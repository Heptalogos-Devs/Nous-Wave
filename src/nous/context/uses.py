"""Record actual context uses through the same authorized event owner."""

from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.context import ContextAdmission, ContextUse
from sqlalchemy import select, update

from nous.platform.persistence import WriteTx

from .feedback_events import record_in
from .models import contexts, sessions

if TYPE_CHECKING:
    from .managed import ManagedContexts


async def record_uses_in(
    self: ManagedContexts,
    tx: WriteTx,
    ctx: CallContext,
    admission: ContextAdmission,
    uses: tuple[ContextUse, ...],
    occurred_at: datetime,
) -> None:
    context_value = await tx.connection.scalar(select(contexts.c.record).where(contexts.c.id == admission.id))
    for use in uses:
        if use.target not in admission.sources:
            raise DomainError(
                "context_use_not_admitted",
                "Reported use was not present in the admitted model input",
            )
        await record_in(self, tx, ctx, context_id=admission.id, event_id=use.event_id,
            target=use.target, kind="actual_use", use_kind=use.kind,
            description=use.description, invocation_id=admission.invocation_id,
            session_id=admission.session_id, occurred_at=occurred_at.isoformat(),
            subject_view=bool(context_value and context_value.get("subject_view", False)))
    if uses:
        from .lifecycle import session_key

        current_session = await self.session_in(tx, ctx, admission.session_id)
        updated_residents = tuple(
            item.model_copy(
                update={
                    "last_meaningful_use_at": max(
                        occurred_at, item.last_meaningful_use_at or item.entered_at
                    )
                }
            )
            if any(use.target == item.target for use in uses)
            else item
            for item in current_session.residents
        )
        revised_session = current_session.model_copy(
            update={
                "residents": updated_residents,
                "revision": current_session.revision + 1,
                "commit_seq": tx.commit_seq,
            }
        )
        await tx.connection.execute(
            update(sessions)
            .where(sessions.c.id == session_key(ctx, admission.session_id))
            .values(record=revised_session.model_dump(mode="json"))
        )
