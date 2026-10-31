"""Read-only directory over the authoritative revisions, with stable keyset pages."""

from __future__ import annotations

import base64
import binascii
import hashlib
from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    DomainError,
    EntityBody,
    ReadRecords,
    RecordDetail,
    RecordEntry,
    RecordPurpose,
    RecordsCoverage,
    RecordsPage,
    RecordsQuery,
)
from nous_sdk.contracts.common import Contract
from pydantic import Field, ValidationError
from sqlalchemy import and_, func, select, tuple_

from nous.identity.access import require_subject_in
from nous.platform.persistence import ReadView, WriteTx
from nous.platform.persistence.authority import canonical_json
from nous.platform.time.filtering import time_matches

from .lifecycle import record_state, selectable
from .projection import body_text, references_in
from .schema import revisions

if TYPE_CHECKING:
    from .service import CognitionService


class _Cursor(Contract):
    binding: str = Field(min_length=64, max_length=64)
    snapshot: int = Field(ge=0)
    sequence: int = Field(ge=0)
    object_id: str = Field(min_length=1)
    revision: int = Field(ge=1)

    def encode(self) -> str:
        return base64.urlsafe_b64encode(self.model_dump_json().encode()).decode().rstrip("=")


def _matches(record: CognitiveRecord, command: RecordsQuery, roots: dict[str, str]) -> bool:
    if command.text is not None:
        wanted = command.text.strip().casefold()
        if command.text_match == "label":
            if (
                not isinstance(record.body, EntityBody)
                or record.body.label.strip().casefold() != wanted
            ):
                return False
        elif wanted not in body_text(record.body).casefold():
            return False
    if command.purpose == "current" and not selectable(record):
        return False
    if command.purpose == "exact" and record.deleted:
        return False
    if command.states and record_state(record) not in command.states:
        return False
    if command.source_ids is not None and not any(
        oid in command.source_ids or roots.get(oid) in command.source_ids
        for oid in record.source_roots
    ):
        return False
    if command.entity_ids:
        entities = {ref.id for ref in references_in(record.body) if ref.kind == "entity"}
        if record.reference.kind == "entity":
            entities.add(record.reference.id)
        if not entities.intersection(command.entity_ids):
            return False
    return time_matches(record, command.times)


async def records_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    command: RecordsQuery,
) -> RecordsPage:
    await require_subject_in(view, ctx)
    if command.subject_id is not None and command.subject_id != ctx.require_subject():
        raise DomainError(
            "subject_scope_mismatch", "Select the subject through the authorized context"
        )
    binding = hashlib.sha256(
        canonical_json(
            (
                ctx.caller_id,
                ctx.namespace_id,
                ctx.require_subject(),
                ctx.source_groups(),
                command.model_dump(mode="json", exclude={"cursor", "limit"}),
            )
        ).encode()
    ).hexdigest()
    position: _Cursor | None = None
    if command.cursor is not None:
        try:
            position = _Cursor.model_validate_json(
                base64.b64decode(
                    command.cursor + "=" * (-len(command.cursor) % 4), altchars=b"-_", validate=True
                )
            )
        except (ValidationError, ValueError, binascii.Error) as error:
            raise DomainError("invalid_cursor", "Record cursor is invalid") from error
        if position.binding != binding:
            raise DomainError(
                "invalid_cursor", "Record cursor belongs to a different query or scope"
            )
    snapshot = (
        position.snapshot
        if position
        else (command.as_of_commit if command.as_of_commit is not None else view.commit_seq)
    )
    if snapshot > view.commit_seq:
        raise DomainError("invalid_history", "Record snapshot is outside committed history")
    query = select(revisions.c.record).where(
        revisions.c.namespace_id == ctx.namespace_id,
        revisions.c.subject_id == ctx.require_subject(),
        revisions.c.commit_seq <= snapshot,
    )
    if command.view == "current":
        latest = (
            select(revisions.c.id, func.max(revisions.c.revision).label("revision"))
            .where(
                revisions.c.namespace_id == ctx.namespace_id,
                revisions.c.subject_id == ctx.require_subject(),
                revisions.c.commit_seq <= snapshot,
            )
            .group_by(revisions.c.id)
            .subquery()
        )
        query = query.join(
            latest, and_(revisions.c.id == latest.c.id, revisions.c.revision == latest.c.revision)
        )
    if command.kinds:
        query = query.where(revisions.c.kind.in_(command.kinds))
    if command.object_ids:
        query = query.where(revisions.c.id.in_(command.object_ids))
    order = (revisions.c.commit_seq, revisions.c.id, revisions.c.revision)
    query = query.order_by(*(column.desc() for column in order))
    roots = await self.source_roots_in(view, ctx, purpose=command.purpose)
    collected: list[CognitiveRecord] = []
    examined = 0
    batch_size = max(128, command.limit + 1)
    exhausted = False
    while len(collected) <= command.limit:
        batch = query
        if position is not None:
            batch = batch.where(
                tuple_(*order) < (position.sequence, position.object_id, position.revision)
            )
        rows = (await view.connection.execute(batch.limit(batch_size))).scalars().all()
        if not rows:
            exhausted = True
            break
        for raw in rows:
            record = CognitiveRecord.model_validate(raw)
            if command.purpose != "management":
                record = await self.project_record_in(view, ctx, record)
            examined += 1
            position = _Cursor(
                binding=binding,
                snapshot=snapshot,
                sequence=record.commit_seq,
                object_id=record.reference.id,
                revision=record.reference.revision,
            )
            if _matches(record, command, roots) and await self.visible_in(
                view, ctx, record, roots, purpose=command.purpose
            ):
                collected.append(record)
                if len(collected) > command.limit:
                    break
        if len(collected) > command.limit:
            break
        if len(rows) < batch_size:
            exhausted = True
            break
    next_cursor = None
    if len(collected) > command.limit:
        last = collected[command.limit - 1]
        next_cursor = _Cursor(
            binding=binding,
            snapshot=snapshot,
            sequence=last.commit_seq,
            object_id=last.reference.id,
            revision=last.reference.revision,
        ).encode()
    records = tuple(collected[: command.limit])
    heads: dict[str, CognitiveRef] = {}
    if records:
        for raw in (
            await view.connection.execute(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.require_subject(),
                    revisions.c.id.in_([record.reference.id for record in records]),
                    revisions.c.head.is_(True),
                )
            )
        ).scalars():
            head = CognitiveRecord.model_validate(raw)
            if await self.visible_in(view, ctx, head, roots, purpose=command.purpose):
                heads[head.reference.id] = head.reference
    return RecordsPage(
        items=tuple(
            RecordEntry(
                record=record,
                current_reference=heads.get(record.reference.id),
                state=record_state(record),
            )
            for record in records
        ),
        next_cursor=next_cursor,
        coverage=RecordsCoverage(
            snapshot_commit=snapshot,
            authorization_commit=view.commit_seq,
            examined=examined,
            returned=len(records),
            complete=exhausted,
        ),
    )


async def records(self: CognitionService, ctx: CallContext, command: RecordsQuery) -> RecordsPage:
    async with self.authority.read(ctx) as view:
        return await self.records_in(view, ctx, command)


async def read_records(
    self: CognitionService, ctx: CallContext, command: ReadRecords
) -> tuple[CognitiveRecord, ...]:
    async with self.authority.read(ctx) as view:
        return await self.read_records_in(view, ctx, command)


async def read_records_in(
    self: CognitionService, view: ReadView | WriteTx, ctx: CallContext, command: ReadRecords
) -> tuple[CognitiveRecord, ...]:
    await require_subject_in(view, ctx)
    values = (
        await view.connection.execute(
            select(revisions.c.record).where(
                revisions.c.namespace_id == ctx.namespace_id,
                revisions.c.subject_id == ctx.require_subject(),
                tuple_(revisions.c.kind, revisions.c.id, revisions.c.revision).in_(
                    [(ref.kind, ref.id, ref.revision) for ref in command.references]
                ),
            )
        )
    ).scalars()
    by_ref = {
        record.reference: record
        for record in (CognitiveRecord.model_validate(value) for value in values)
    }
    heads: dict[str, int] = {}
    if command.purpose == "current":
        heads = {
            str(row.id): int(row.revision)
            for row in (
                await view.connection.execute(
                    select(revisions.c.id, revisions.c.revision).where(
                        revisions.c.namespace_id == ctx.namespace_id,
                        revisions.c.subject_id == ctx.require_subject(),
                        revisions.c.id.in_([ref.id for ref in command.references]),
                        revisions.c.head.is_(True),
                    )
                )
            ).all()
        }
    roots = await self.source_roots_in(view, ctx, purpose=command.purpose)
    result: list[CognitiveRecord] = []
    for reference in command.references:
        record = by_ref.get(reference)
        if record is None:
            raise DomainError("cognition_not_found", "Cognitive revision is unavailable")
        if command.purpose != "management":
            record = await self.project_record_in(view, ctx, record)
        if command.purpose == "current" and (
            heads.get(reference.id) != reference.revision or not selectable(record)
        ):
            raise DomainError(
                "cognition_inactive", "The revision is not current available cognition"
            )
        if command.purpose != "management" and record.deleted:
            raise DomainError("cognition_deleted", "Deleted revisions require management reading")
        if not await self.visible_in(view, ctx, record, roots, purpose=command.purpose):
            raise DomainError(
                "source_unavailable", "Cognitive sources are unavailable in this scope"
            )
        result.append(record)
    return tuple(result)


async def record_detail(
    self: CognitionService,
    ctx: CallContext,
    reference: CognitiveRef,
    history_limit: int = 100,
    purpose: RecordPurpose = "exact",
) -> RecordDetail:
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx)
        record = await self.read_in(view, ctx, reference, purpose=purpose)
        if purpose != "management" and record.deleted:
            raise DomainError("cognition_deleted", "Deleted revisions require management reading")
        history = await self.records_in(
            view,
            ctx,
            RecordsQuery(
                object_ids=(reference.id,),
                view="history",
                purpose=purpose,
                limit=history_limit,
            ),
        )
        current = next((entry.current_reference for entry in history.items), None)
        return RecordDetail(
            entry=RecordEntry(record=record, current_reference=current, state=record_state(record)),
            history=history,
        )
