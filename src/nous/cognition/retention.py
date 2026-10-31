"""One retention transition owner and optimistic state checks for supported cognitive domains."""

from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    DomainError,
    EvidenceLink,
    RetentionChange,
    RetentionChanges,
    RetentionExpectation,
    RevisionCause,
)
from nous_sdk.contracts.common import utc_now
from nous_sdk.contracts.records import RecordsQuery
from nous_sdk.contracts.retention import DecayRetention, RetentionDecayResult
from sqlalchemy import select

from nous.identity.access import require_subject_in
from nous.platform.persistence import ReadView, WriteTx

from .lifecycle import snapshot, state_for, transition
from .revision_state import RevisionState
from .schema import revisions, vectors

if TYPE_CHECKING:
    from .service import CognitionService


async def require_preconditions_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    expectations: tuple[RetentionExpectation, ...],
) -> None:
    if not expectations:
        return
    await require_subject_in(view, ctx)
    for expected in expectations:
        raw = await view.connection.scalar(
            select(revisions.c.record).where(
                revisions.c.namespace_id == ctx.namespace_id,
                revisions.c.subject_id == ctx.require_subject(),
                revisions.c.kind == expected.reference.kind,
                revisions.c.id == expected.reference.id,
                revisions.c.head.is_(True),
            )
        )
        if raw is None:
            raise DomainError(
                "retention_precondition_failed",
                "The expected retained object is no longer present",
                reference=expected.reference.exact(),
            )
        record = CognitiveRecord.model_validate(raw)
        await self.get_in(view, ctx, record.reference, purpose="management")
        if record.purged or record.reference.kind not in {"memory", "association"}:
            raise DomainError(
                "retention_precondition_failed",
                "The expected retained object is unavailable",
                reference=expected.reference.exact(),
            )
        # Score is time-dependent; the exact stored state and committed reference
        # protect the transaction, rather than a second mutable revision counter.
        actual = snapshot(
            record,
            int(datetime.fromisoformat(view.recorded_at).timestamp() * 1000)
            if isinstance(view, WriteTx)
            else utc_now().coordinate,
        )
        actual_fields = actual.model_dump(mode="json", exclude={"score"})
        wanted = expected.model_dump(mode="json")
        if actual_fields != wanted:
            raise DomainError(
                "retention_precondition_failed",
                "Retention state changed after selection",
                reference=expected.reference.exact(),
                current=record.reference.exact(),
                fields=[name for name in wanted if actual_fields.get(name) != wanted[name]],
            )


async def change_in(
    self: CognitionService, tx: WriteTx, ctx: CallContext, command: RetentionChange
) -> CognitiveRecord:
    previous = await self.get_in(tx, ctx, command.reference)
    lifecycle = state_for(previous)
    current = await tx.connection.scalar(
        select(revisions.c.revision).where(
            revisions.c.id == previous.reference.id,
            revisions.c.namespace_id == ctx.namespace_id,
            revisions.c.subject_id == ctx.subject_id,
            revisions.c.head.is_(True),
        )
    )
    if current != command.reference.revision:
        raise DomainError(
            "retention_revision_conflict",
            "The retained object changed after selection",
            reference=command.reference.exact(),
        )
    if previous.deleted and command.action not in {"restore", "purge"}:
        raise DomainError("retention_deleted", "Restore the retained object before changing it")
    if command.hours is not None and command.action != "protect":
        raise DomainError("invalid_retention_hours", "Hours applies to protection")
    now = int(datetime.fromisoformat(tx.recorded_at).timestamp() * 1000)
    if command.action in {"trash", "purge"} and lifecycle.protected(now) and not command.force:
        raise DomainError(
            "retention_protected",
            "The retained object is protected; unprotect it or explicitly force the change",
        )
    updated = transition(lifecycle, command, now)
    deleted = (
        command.action in {"trash", "purge"}
        if command.action in {"trash", "purge", "restore"}
        else previous.deleted
    )
    if updated == lifecycle and deleted == previous.deleted and command.action != "purge":
        return previous
    assert previous.body is not None
    result = await self.adopt_in(
        tx,
        ctx,
        AdoptCognition(
            id=previous.reference.id,
            expected_revision=previous.reference.revision,
            body=previous.body,
            evidence=(
                EvidenceLink(reference=previous.reference, role=command.reason or command.action),
            ),
            times=previous.times,
            producer_call_id=previous.producer_call_id,
        ),
        state=RevisionState(
            lifecycle=updated,
            deleted=deleted,
            purged=command.action == "purge",
            cause=RevisionCause(
                action=command.action,
                actor=ctx.caller_id,
                reason=command.reason,
                parent=previous.reference,
            ),
        ),
    )
    if command.action == "purge":
        await self.erase_history_in(tx, ctx, previous.reference.id)
    else:
        for asset in (
            await tx.connection.execute(
                select(vectors).where(vectors.c.reference == previous.reference.exact())
            )
        ).mappings():
            await tx.connection.execute(
                vectors.insert().values(
                    **{**asset, "reference": result.reference.exact(), "commit_seq": tx.commit_seq}
                )
            )
    return result


async def change_many_in(
    self: CognitionService, tx: WriteTx, ctx: CallContext, command: RetentionChanges
) -> tuple[CognitiveRecord, ...]:
    if len({ref.id for ref in command.references}) != len(command.references):
        raise DomainError("retention_selection", "Select each retained identity once")
    for reference in command.references:
        record = await self.get_in(tx, ctx, reference)
        state_for(record)
        current = await tx.connection.scalar(
            select(revisions.c.revision).where(
                revisions.c.id == reference.id,
                revisions.c.namespace_id == ctx.namespace_id,
                revisions.c.subject_id == ctx.subject_id,
                revisions.c.head.is_(True),
            )
        )
        if current != reference.revision:
            raise DomainError(
                "retention_revision_conflict",
                "The frozen lifecycle selection changed",
                reference=reference.exact(),
            )
    return tuple(
        [
            await change_in(
                self,
                tx,
                ctx,
                RetentionChange(
                    reference=reference,
                    action=command.action,
                    strength=command.strength,
                    hours=command.hours,
                    reason=command.reason,
                    force=command.force,
                ),
            )
            for reference in command.references
        ]
    )


async def change(
    self: CognitionService, ctx: CallContext, command: RetentionChange
) -> CognitiveRecord:
    async with self.authority.write(
        ctx,
        command.operation_id,
        operation_kind="retention.change",
        scope=ctx.require_subject(),
        payload=command,
    ) as tx:
        if tx.replayed:
            return await self.replay_record_in(
                tx, ctx, CognitiveRecord.model_validate(tx.replay_result)
            )
        result = tx.result(await change_in(self, tx, ctx, command))
    if self.invalidate:
        await self.invalidate()
    return result


async def change_many(
    self: CognitionService, ctx: CallContext, command: RetentionChanges
) -> tuple[CognitiveRecord, ...]:
    from pydantic import TypeAdapter

    async with self.authority.write(
        ctx,
        command.operation_id,
        operation_kind="retention.change_many",
        scope=ctx.require_subject(),
        payload=command,
    ) as tx:
        if tx.replayed:
            return await self.replay_records_in(
                tx, ctx, TypeAdapter(tuple[CognitiveRecord, ...]).validate_python(tx.replay_result)
            )
        result = tx.result(await change_many_in(self, tx, ctx, command))
    if self.invalidate:
        await self.invalidate()
    return result


async def read_snapshot(self: CognitionService, ctx: CallContext, reference: CognitiveRef):
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx)
        return snapshot(
            await self.get_in(view, ctx, reference, purpose="management"), utc_now().coordinate
        )


async def decay_in(self: CognitionService, tx: WriteTx, ctx: CallContext, command: DecayRetention) -> RetentionDecayResult:
    page = await self.records_in(tx, ctx, RecordsQuery(kinds=command.kinds, limit=command.limit, cursor=command.cursor))
    now = int(datetime.fromisoformat(tx.recorded_at).timestamp() * 1000)
    changed: list[CognitiveRef] = []
    for entry in page.items:
        before = state_for(entry.record)
        change = RetentionChange(reference=entry.record.reference, action="decay", reason="retention evaluation")
        if transition(before, change, now) != before:
            changed.append((await change_in(self, tx, ctx, change)).reference)
    return RetentionDecayResult(scanned=len(page.items), changed=tuple(changed), next_cursor=page.next_cursor, complete=page.next_cursor is None)


async def decay(self: CognitionService, ctx: CallContext, command: DecayRetention) -> RetentionDecayResult:
    async with self.authority.write(ctx, command.operation_id, operation_kind="retention.decay", scope=ctx.require_subject(), payload=command) as tx:
        if tx.replayed:
            return RetentionDecayResult.model_validate(tx.replay_result)
        result = tx.result(await decay_in(self, tx, ctx, command))
    if self.invalidate:
        await self.invalidate()
    return result
