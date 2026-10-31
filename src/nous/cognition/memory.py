"""Memory operations share the cognitive authority transaction."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts.lifecycle import MemoryChanges
from pydantic import TypeAdapter
from sqlalchemy import select

from nous.identity.access import require_subject_in
from nous.platform.persistence import WriteTx

from .revision_state import RevisionState
from .schema import revisions, vectors

if TYPE_CHECKING:
    from .service import CognitionService

from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    CognitiveRecord,
    DomainError,
    EvidenceLink,
    ImportDocument,
    MemoryBody,
    MemoryChange,
    RetentionChange,
    RetentionState,
    RevisionCause,
    TimePoint,
)


async def change_memory_in(
    self: CognitionService,
    tx: WriteTx,
    ctx: CallContext,
    command: MemoryChange,
    *,
    revision_evidence: tuple[EvidenceLink, ...] = (),
) -> CognitiveRecord:
    if command.reference.kind != "memory":
        raise DomainError("memory_required", "Memory operations require a Memory reference")
    if command.action != "revise":
        return await self.change_retention_in(
            tx,
            ctx,
            RetentionChange(
                reference=command.reference,
                action=command.action,
                strength=command.strength,
                hours=command.hours,
                reason=command.reason,
                force=command.force,
            ),
        )

    previous = await self.get_in(tx, ctx, command.reference)
    if (
        previous.reference.kind != "memory"
        or previous.purged
        or not isinstance(previous.body, MemoryBody)
    ):
        raise DomainError("memory_unavailable", "Maintenance requires a nonpurged Memory revision")
    if previous.deleted:
        raise DomainError("memory_deleted", "Restore the memory before changing it")
    if command.body is None:
        raise DomainError("invalid_memory_edit", "Revise requires the complete edited memory body")
    if not any(link.polarity == "support" for link in revision_evidence):
        raise DomainError("memory_revision_source_required", "Admit the actual edited input first")
    body = command.body or previous.body
    content_changed = (
        body.text != previous.body.text
        or body.summary != previous.body.summary
        or body.structured != previous.body.structured
        or body.epistemic != previous.body.epistemic
        or body.role != previous.body.role
        or body.formation_mode != previous.body.formation_mode
    )
    if content_changed:
        body = body.model_copy(
            update={
                "media": tuple(
                    reference for reference in body.media if reference not in previous.body.media
                ),
                "contact_observation_id": None
                if body.contact_observation_id == previous.body.contact_observation_id
                else body.contact_observation_id,
            }
        )
    times = previous.times
    if content_changed:
        from nous.material.service import require_observation_in

        observed: list[TimePoint | None] = []
        for link in revision_evidence:
            if link.polarity != "support":
                continue
            if link.reference is not None:
                observed.append((await self.get_in(tx, ctx, link.reference)).times.observation)
            elif link.observation_id is not None:
                source = await require_observation_in(
                    tx, ctx, link.observation_id, region_id=link.region_id
                )
                observed.append(TimePoint.model_validate(source["record"]["observed_at"]))
        known = [point for point in observed if point is not None]
        observed_at = (
            max(known, key=lambda point: point.coordinate)
            if known and len(known) == len(observed) and len({point.domain for point in known}) == 1
            else None
        )
        times = times.model_copy(update={"formation": ctx.clock, "observation": observed_at})
    edit_evidence = revision_evidence
    if not content_changed:
        edit_evidence = (
            *previous.evidence,
            *(
                link
                if link.reference is not None
                else link.model_copy(update={"polarity": "boundary"})
                for link in revision_evidence
            ),
        )
    lifecycle = previous.lifecycle or RetentionState(
        anchor_unix_ms=previous.times.recorded.coordinate
    )
    state = RevisionState(
        lifecycle=lifecycle,
        cause=RevisionCause(
            action=command.action,
            actor=ctx.caller_id,
            reason=command.reason,
            parent=previous.reference,
        ),
    )
    result = await self.adopt_in(
        tx,
        ctx,
        AdoptCognition(
            id=previous.reference.id,
            expected_revision=command.reference.revision,
            body=body,
            evidence=(
                *edit_evidence,
                EvidenceLink(
                    reference=previous.reference, polarity="boundary", role="previous content"
                ),
            ),
            times=times,
            producer_call_id=None,
        ),
        state=state,
    )
    return result


async def revise_source_times_in(
    self: CognitionService,
    tx: WriteTx,
    ctx: CallContext,
    previous: CognitiveRecord,
    *,
    event: TimePoint | None = None,
    observation: TimePoint | None = None,
) -> CognitiveRecord:
    await self.get_in(tx, ctx, previous.reference)
    if not isinstance(previous.body, MemoryBody):
        raise DomainError("memory_required", "Source chronology applies to Memory")
    changes: dict[str, TimePoint] = {}
    if previous.times.event is None and event is not None:
        changes["event"] = event
    if previous.times.observation is None and observation is not None:
        changes["observation"] = observation
    if not changes:
        return previous
    result = await self.adopt_in(
        tx,
        ctx,
        AdoptCognition(
            id=previous.reference.id,
            expected_revision=previous.reference.revision,
            body=previous.body,
            evidence=(
                *previous.evidence,
                EvidenceLink(
                    reference=previous.reference, polarity="boundary", role="source chronology"
                ),
            ),
            times=previous.times.model_copy(update=changes),
            producer_call_id=previous.producer_call_id,
        ),
        state=RevisionState(
            cause=RevisionCause(
                action="memory.source_times",
                actor=ctx.caller_id,
                reason="Recovered explicit source time fields",
                parent=previous.reference,
            ),
            lifecycle=previous.lifecycle,
            deleted=previous.deleted,
        ),
    )
    assets = (
        (
            await tx.connection.execute(
                select(vectors).where(vectors.c.reference == previous.reference.exact())
            )
        )
        .mappings()
        .all()
    )
    for asset in assets:
        await tx.connection.execute(
            vectors.insert().values(
                **{**asset, "reference": result.reference.exact(), "commit_seq": tx.commit_seq}
            )
        )
    return result


async def change_memory(
    self: CognitionService, ctx: CallContext, command: MemoryChange
) -> CognitiveRecord:
    document = (
        ImportDocument(
            text=command.model_dump_json(),
            format="json",
            filename="memory-revision.json",
            media_type="application/json",
            source_type="administrative_change",
            source="memory_management",
            origin={"caller_id": ctx.caller_id},
        )
        if command.action == "revise"
        else None
    )
    store = self.materials.content_store
    lease = (
        await store.publish_bytes(document.inline_text().encode(), document.media_type)
        if document is not None and store
        else None
    )
    try:
        async with self.authority.write(
            ctx,
            command.operation_id,
            operation_kind="memory.change",
            scope=ctx.require_subject(),
            payload=command,
        ) as tx:
            if tx.replayed:
                result = CognitiveRecord.model_validate(tx.replay_result)
                if document is not None and ctx.source_groups():
                    admission = await self.materials.restore_input_in(
                        tx,
                        ctx,
                        operation_id=command.operation_id,
                        operation_kind="memory.change",
                        scope=ctx.require_subject(),
                    )
                    effective = await self.qualify_known_outputs_in(
                        tx,
                        ctx,
                        (result.reference,),
                        operation_id=command.operation_id,
                        operation_kind="memory.change",
                        scope=ctx.require_subject(),
                        input_material_ids=admission.material_ids,
                    )
                else:
                    effective = ctx
                return await self.replay_record_in(tx, effective, result)
            effective, revision_evidence = ctx, ()
            if document is not None:
                await self.get_in(tx, ctx, command.reference)
                admission = await self.materials.admit_text_in(tx, ctx, document, lease)
                effective = admission.context
                revision_evidence = (
                    EvidenceLink(observation_id=admission.event["id"], role="memory_revision"),
                )
            result = tx.result(
                await change_memory_in(
                    self, tx, effective, command, revision_evidence=revision_evidence
                )
            )
    finally:
        if lease and store:
            await store.release(lease)
    if self.invalidate:
        await self.invalidate()
    return result


async def recycle_bin(
    self: CognitionService, ctx: CallContext, limit: int = 50
) -> tuple[CognitiveRecord, ...]:
    if not 1 <= limit <= 1000:
        raise DomainError("invalid_limit", "Recycle-bin limit must be between 1 and 1000")
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx, "memory")
        result: list[CognitiveRecord] = []
        roots = await self.source_roots_in(view, ctx)
        for value in (
            await view.connection.execute(
                select(revisions.c.record)
                .where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.kind == "memory",
                    revisions.c.head.is_(True),
                )
                .order_by(revisions.c.commit_seq.desc())
            )
        ).scalars():
            record = CognitiveRecord.model_validate(value)
            if record.deleted and await self.visible_in(view, ctx, record, roots):
                result.append(record)
            if len(result) >= limit:
                break
        return tuple(result)


async def decay_memories(self: CognitionService, ctx: CallContext, operation_id: str, limit: int = 1000):
    from nous_sdk.contracts.retention import DecayRetention
    async with self.authority.write(ctx, operation_id, operation_kind="memory.decay", scope=ctx.require_subject(), payload={"limit": limit}) as tx:
        if tx.replayed:
            return tx.replay_result
        await require_subject_in(tx, ctx, "memory")
        result = await self.decay_retention_in(tx, ctx, DecayRetention(kinds=("memory",), limit=limit))
        return tx.result({"scanned": result.scanned, "changed": [ref.exact() for ref in result.changed], "complete": result.complete, "next_cursor": result.next_cursor})


async def change_memories_in(self: CognitionService, tx: WriteTx, ctx: CallContext, command: MemoryChanges) -> tuple[CognitiveRecord, ...]:
    from nous_sdk.contracts import RetentionChanges
    if any(reference.kind != "memory" for reference in command.references):
        raise DomainError("memory_required", "Memory operations require Memory references")
    return await self.change_retentions_in(tx, ctx, RetentionChanges.model_validate(command.model_dump()))


async def change_memories(
    self: CognitionService, ctx: CallContext, command: MemoryChanges
) -> tuple[CognitiveRecord, ...]:
    """Execute a frozen selection once; no repeated query can change its membership."""
    async with self.authority.write(
        ctx,
        command.operation_id,
        operation_kind="memory.change_many",
        scope=ctx.require_subject(),
        payload=command,
    ) as tx:
        if tx.replayed:
            return await self.replay_records_in(
                tx, ctx, TypeAdapter(tuple[CognitiveRecord, ...]).validate_python(tx.replay_result)
            )
        result = tx.result(await self.change_memories_in(tx, ctx, command))
    if self.invalidate:
        await self.invalidate()
    return result
