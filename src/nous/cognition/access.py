"""Public history and identity resolution use the same source visibility as reads."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    ConceptBody,
    DomainError,
    EvidenceLink,
    MediaReference,
    MemoryBody,
    MemoryChange,
)
from sqlalchemy import select

from nous.identity.access import require_subject_in
from nous.platform.persistence import ReadView, WriteTx

from .projection import evidence_in, media_references_in
from .revision_state import RevisionState
from .schema import revisions

if TYPE_CHECKING:
    from .service import CognitionService


async def support_available_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    record: CognitiveRecord,
    roots: dict[str, str],
    path: frozenset[str] = frozenset(),
    *,
    withdrawn_observations: frozenset[str] = frozenset(),
    purged_observations: frozenset[str] = frozenset(),
    purged_cognition: frozenset[str] = frozenset(),
) -> bool:
    """Inspect declared proof liveness; caller source permissions remain with visible_in."""
    identity = record.reference.exact()
    if (
        record.purged
        or record.reference.id in purged_cognition
        or identity in purged_cognition
        or identity in path
        or not set(record.support_roots or record.source_roots).intersection(roots)
    ):
        return False
    if isinstance(record.body, MemoryBody) and record.body.contact_observation_id is not None:
        if record.body.contact_observation_id not in roots:
            return False
    media = tuple(media_references_in(record.body))
    for reference in media:
        if (
            await self.derivations.availability_in(
                view,
                ctx,
                reference,
                withdrawn_observations=withdrawn_observations,
                purged_observations=purged_observations,
                purged_cognition=purged_cognition,
            )
            != "available"
        ):
            return False
    if media:
        return True
    links = tuple(link for link in record.evidence if link.polarity == "support") or tuple(
        link for link in evidence_in(record.body) if link.polarity == "support"
    )
    for link in links:
        if link.observation_id in roots:
            assert link.observation_id is not None
            source = MediaReference(
                kind="source_region" if link.region_id else "material",
                id=link.region_id or roots[link.observation_id],
                observation_id=link.observation_id,
            )
            if (
                await self.derivations.availability_in(
                    view,
                    ctx,
                    source,
                    withdrawn_observations=withdrawn_observations,
                    purged_observations=purged_observations,
                    purged_cognition=purged_cognition,
                )
                == "available"
            ):
                return True
        if link.reference is not None:
            raw = await view.connection.scalar(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.kind == link.reference.kind,
                    revisions.c.id == link.reference.id,
                    revisions.c.revision == link.reference.revision,
                )
            )
            if raw is not None and await support_available_in(
                self,
                view,
                ctx,
                CognitiveRecord.model_validate(raw),
                roots,
                path | {identity},
                withdrawn_observations=withdrawn_observations,
                purged_observations=purged_observations,
                purged_cognition=purged_cognition,
            ):
                return True
    return False


async def history(
    self: CognitionService, ctx: CallContext, object_id: str, limit: int = 100
) -> tuple[CognitiveRecord, ...]:
    if not 1 <= limit <= 1000:
        raise DomainError("invalid_limit", "History limit must be between 1 and 1000")
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx)
        values = (
            await view.connection.execute(
                select(revisions.c.record)
                .where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.id == object_id,
                )
                .order_by(revisions.c.revision.desc())
                .limit(limit)
            )
        ).scalars()
        result: list[CognitiveRecord] = []
        for value in values:
            record = await self.project_record_in(view, ctx, CognitiveRecord.model_validate(value))
            if await self.visible_in(view, ctx, record):
                result.append(record)
        return tuple(result)


async def concept_by_alias(self: CognitionService, ctx: CallContext, label: str) -> CognitiveRecord:
    key = " ".join(label.casefold().split())
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx)
        values = (
            await view.connection.execute(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.kind == "concept",
                    revisions.c.head.is_(True),
                )
            )
        ).scalars()
        matches: list[CognitiveRecord] = []
        for value in values:
            record = CognitiveRecord.model_validate(value)
            if (
                isinstance(record.body, ConceptBody)
                and not record.deleted
                and record.body.state == "active"
                and key
                in {
                    " ".join(term.casefold().split())
                    for term in (record.body.label, *record.body.aliases)
                }
                and await self.visible_in(view, ctx, record)
            ):
                matches.append(record)
        if not matches:
            raise DomainError("concept_not_found", "No accessible Concept matches this alias")
        if len(matches) > 1:
            raise DomainError("ambiguous_name", "The alias identifies multiple accessible Concepts")
        return matches[0]


async def set_deleted(
    self: CognitionService,
    ctx: CallContext,
    reference: CognitiveRef,
    deleted: bool,
    operation_id: str,
) -> CognitiveRecord:
    async with self.authority.read(ctx) as view:
        previous = await self.get_in(view, ctx, reference)
    if isinstance(previous.body, MemoryBody):
        return await self.change_memory(
            ctx,
            MemoryChange(
                operation_id=operation_id,
                reference=reference,
                action="trash" if deleted else "restore",
            ),
        )
    async with self.authority.write(
        ctx,
        operation_id,
        operation_kind="cognition.set_deleted",
        scope=ctx.require_subject(),
        payload={"reference": reference.model_dump(mode="json"), "deleted": deleted},
    ) as tx:
        if tx.replayed:
            return await self.replay_record_in(
                tx, ctx, CognitiveRecord.model_validate(tx.replay_result)
            )
        adopted = await self.set_deleted_in(tx, ctx, reference, deleted)
        result = tx.result(adopted)
    if self.invalidate:
        await self.invalidate()
    return result


async def set_deleted_in(
    self: CognitionService,
    tx: WriteTx,
    ctx: CallContext,
    reference: CognitiveRef,
    deleted: bool,
    *,
    evidence: tuple[EvidenceLink, ...] = (),
    reason: str = "Manual cognitive lifecycle change",
) -> CognitiveRecord:
    previous = await self.get_in(tx, ctx, reference)
    if previous.reference.kind in {"memory", "association"}:
        from nous_sdk.contracts import RetentionChange

        return await self.change_retention_in(
            tx,
            ctx,
            RetentionChange(
                reference=reference, action="trash" if deleted else "restore", reason=reason
            ),
        )
    if previous.body is None:
        raise DomainError("cognition_purged", "A purged object cannot be changed")
    from nous_sdk.contracts import RevisionCause

    return await self.adopt_in(
        tx,
        ctx,
        AdoptCognition(
            id=reference.id,
            expected_revision=reference.revision,
            body=previous.body,
            evidence=(
                *previous.evidence,
                EvidenceLink(reference=reference, polarity="boundary", role=reason),
                *(link.model_copy(update={"polarity": "boundary"}) for link in evidence),
            ),
            times=previous.times,
            producer_call_id=previous.producer_call_id,
        ),
        state=RevisionState(
            deleted=deleted,
            lifecycle=previous.lifecycle,
            cause=RevisionCause(
                action="trash" if deleted else "restore",
                actor=ctx.caller_id,
                reason=reason,
                parent=reference,
            ),
        ),
    )


async def replay_record_in(
    self: CognitionService, view: ReadView | WriteTx, ctx: CallContext, record: CognitiveRecord
) -> CognitiveRecord:
    """An operation receipt never preserves an older, broader source authorization."""
    return await self.get_in(view, ctx, record.reference)


async def automatic_in(
    self: CognitionService, view: ReadView | WriteTx, ctx: CallContext, reference: CognitiveRef
) -> CognitiveRecord | None:
    """Automatic uses retain exact provenance and honor the current domain decision."""
    from .lifecycle import selectable

    exact = await self.read_in(view, ctx, reference)
    if not selectable(exact):
        return None
    raw = await view.connection.scalar(
        select(revisions.c.record).where(
            revisions.c.namespace_id == ctx.namespace_id,
            revisions.c.subject_id == ctx.require_subject(),
            revisions.c.id == reference.id,
            revisions.c.head.is_(True),
        )
    )
    if raw is None:
        return None
    head = await self.project_record_in(view, ctx, CognitiveRecord.model_validate(raw))
    if not selectable(head) or not await self.visible_in(view, ctx, head):
        return None
    return exact


async def replay_records_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    records: tuple[CognitiveRecord, ...],
) -> tuple[CognitiveRecord, ...]:
    return tuple([await self.replay_record_in(view, ctx, record) for record in records])
