"""Source attachment changes are revisions of the actual supported object."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    DomainError,
    EvidenceLink,
    RevisionCause,
)
from nous_sdk.contracts.fact_body import FactClaimBody, FactEvidence

from nous.platform.persistence import WriteTx

from .revision_state import RevisionState

if TYPE_CHECKING:
    from .service import CognitionService


async def revise_evidence_in(
    self: CognitionService,
    tx: WriteTx,
    ctx: CallContext,
    reference: CognitiveRef,
    *,
    add: tuple[EvidenceLink, ...] = (),
    remove: tuple[EvidenceLink, ...] = (),
    reason: str,
) -> CognitiveRecord:
    previous = await self.get_in(tx, ctx, reference)
    if previous.body is None:
        raise DomainError("cognition_purged", "A purged object cannot change evidence")
    if isinstance(previous.body, FactClaimBody):
        if self.fact_owner is None:
            raise RuntimeError("Fact evidence requires the Fact owner")
        return await self.fact_owner.revise_evidence_in(
            tx, ctx, previous, add=add, remove=remove, reason=reason
        )
    return await persist_in(self, tx, ctx, previous, add=add, remove=remove, reason=reason)


async def persist_in(
    cognition: CognitionService,
    tx: WriteTx,
    ctx: CallContext,
    previous: CognitiveRecord,
    *,
    add: tuple[EvidenceLink, ...],
    remove: tuple[EvidenceLink, ...],
    reason: str,
    fact: bool = False,
) -> CognitiveRecord:
    body = previous.body
    assert body is not None
    links = tuple(
        dict.fromkeys((*(link for link in previous.evidence if link not in remove), *add))
    )
    if isinstance(body, FactClaimBody):
        if not fact:
            raise RuntimeError("Fact evidence belongs to its domain owner")
        body = body.model_copy(
            update={
                "evidence": tuple(item for item in body.evidence if item.link not in remove)
                + tuple(
                    FactEvidence(link=link)
                    for link in add
                    if not any(item.link == link for item in body.evidence)
                )
            }
        )
    if body == previous.body and links == previous.evidence:
        return previous
    return await cognition.adopt_in(
        tx,
        ctx,
        AdoptCognition(
            id=previous.reference.id,
            expected_revision=previous.reference.revision,
            body=body,
            evidence=(
                *links,
                EvidenceLink(
                    reference=previous.reference, polarity="boundary", role="evidence history"
                ),
            ),
            times=previous.times,
            producer_call_id=previous.producer_call_id,
        ),
        state=RevisionState(
            deleted=previous.deleted,
            lifecycle=previous.lifecycle,
            cause=RevisionCause(
                action="evidence.change",
                actor=ctx.caller_id,
                reason=reason,
                parent=previous.reference,
            ),
        ),
        fact_owner=cognition.fact_owner if fact else None,
    )
