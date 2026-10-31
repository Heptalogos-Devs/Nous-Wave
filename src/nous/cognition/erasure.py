"""Cognitive erasure shares one history, projection and receipt removal rule."""

from datetime import datetime
from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    DomainError,
    MemoryBody,
    MemoryChange,
)
from pydantic import JsonValue
from sqlalchemy import select, update

from nous.platform.persistence import WriteTx

from .schema import revisions, vectors

if TYPE_CHECKING:
    from .service import CognitionService


async def erase_history_in(
    self: CognitionService, tx: WriteTx, ctx: CallContext, identity: str
) -> None:
    history = [
        CognitiveRecord.model_validate(raw)
        for raw in (
            await tx.connection.execute(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.id == identity,
                )
            )
        ).scalars()
    ]
    for record in history:
        tombstone = record.model_copy(update={"body": None, "purged": True, "deleted": True})
        await tx.connection.execute(
            update(revisions)
            .where(revisions.c.id == identity, revisions.c.revision == record.reference.revision)
            .values(record=tombstone.model_dump(mode="json"))
        )
    refs = tuple(record.reference for record in history) + await self.retire_unsupported_in(tx, ctx)
    await tx.connection.execute(
        vectors.delete().where(vectors.c.reference.in_([ref.exact() for ref in refs]))
    )
    if self.purge_context_in:
        await self.purge_context_in(tx, ctx, refs)

    def contains(value: JsonValue) -> bool:
        if isinstance(value, list):
            return any(contains(item) for item in value)
        if not isinstance(value, dict):
            return False
        reference = value.get("reference")
        return (
            isinstance(reference, dict) and reference.get("id") == identity and "body" in value
        ) or any(contains(item) for item in value.values())

    await self.authority.redact_receipts_in(tx, contains)


async def purge_cognition_in(
    self: CognitionService,
    tx: WriteTx,
    ctx: CallContext,
    reference: CognitiveRef,
    *,
    force: bool = False,
    reason: str,
) -> CognitiveRecord:
    previous = await self.get_in(tx, ctx, reference, purpose="management")
    if previous.purged:
        return previous
    if isinstance(previous.body, MemoryBody) and await self.visible_in(tx, ctx, previous):
        return await self.change_memory_in(
            tx, ctx, MemoryChange(reference=reference, action="purge", force=force, reason=reason)
        )
    if isinstance(previous.body, MemoryBody) and previous.lifecycle is not None:
        now = round(datetime.fromisoformat(tx.recorded_at).timestamp() * 1000)
        if previous.lifecycle.protected(now) and not force:
            raise DomainError(
                "memory_protected", "Memory is protected; unprotect or explicitly force the change"
            )
    await self.erase_history_in(tx, ctx, reference.id)
    return previous.model_copy(update={"body": None, "purged": True, "deleted": True})
