"""Person identity changes create new claim identities and retain old attribution."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, CognitiveRecord, CognitiveRef, DomainError, EvidenceLink
from nous_sdk.contracts.fact_body import FactClaimBody, PersonFactScope
from nous_sdk.contracts.facts import FactChange

from nous.platform.persistence import WriteTx

from .fact_changes import merged_evidence
from .fact_identity import ClaimIdentity, claim_identity, conflicts_with
from .fact_reads import heads_in

if TYPE_CHECKING:
    from .facts import FactService


async def migrate_person_scope_in(
    self: FactService,
    tx: WriteTx,
    ctx: CallContext,
    sources: tuple[CognitiveRef, ...],
    target: CognitiveRef,
    reason: str,
    *,
    claims: tuple[CognitiveRef, ...] | None = None,
) -> tuple[tuple[CognitiveRecord, ...], dict[str, CognitiveRef]]:
    await self.profiles.person_in(tx, ctx, target)
    selected = {ref.id: ref for ref in claims} if claims is not None else None
    found: dict[str, CognitiveRecord] = {}
    groups: dict[ClaimIdentity, list[CognitiveRecord]] = {}
    for source in sources:
        for before in await heads_in(tx, ctx, PersonFactScope(person=source)):
            if (
                before.deleted
                or before.purged
                or not isinstance(before.body, FactClaimBody)
                or before.body.status not in {"active", "conflicted"}
            ):
                continue
            if selected is not None and before.reference.id not in selected:
                continue
            if selected is not None and selected[before.reference.id] != before.reference:
                raise DomainError(
                    "revision_conflict", "A selected claim changed before identity migration"
                )
            await self.cognition.get_in(tx, ctx, before.reference)
            body = before.body.model_copy(update={"scope": PersonFactScope(person=target)})
            groups.setdefault(claim_identity(body, before.times.validity), []).append(before)
            found[before.reference.id] = before
    if selected is not None and set(selected) != set(found):
        raise DomainError(
            "entity_claim_assignment",
            "Each assigned claim must be a current claim of the split entity",
        )
    changes: list[CognitiveRecord] = []
    mapping: dict[str, CognitiveRef] = {}
    report = EvidenceLink(reference=target, polarity="boundary", role=reason)
    incoming = [
        (
            record.body.model_copy(update={"scope": PersonFactScope(person=target)}),
            record.times.validity,
        )
        for values in groups.values()
        for record in values
        if isinstance(record.body, FactClaimBody)
    ]
    for key, originals in groups.items():
        before = originals[0]
        assert isinstance(before.body, FactClaimBody)
        body = before.body.model_copy(
            update={
                "scope": PersonFactScope(person=target),
                "supersedes": (),
                "identity_lineage": tuple(
                    dict.fromkeys(
                        ref
                        for record in originals
                        if isinstance(record.body, FactClaimBody)
                        for ref in (*record.body.identity_lineage, record.reference)
                    )
                ),
            }
        )
        candidates = [
            record
            for record in await heads_in(tx, ctx, body.scope)
            if not record.deleted and not record.purged and isinstance(record.body, FactClaimBody)
        ]
        previous = next(
            (
                record
                for record in candidates
                if isinstance(record.body, FactClaimBody)
                and claim_identity(record.body, record.times.validity) == key
            ),
            None,
        )
        evidence = ()
        for record in originals:
            assert isinstance(record.body, FactClaimBody)
            if record.body.cardinality != body.cardinality:
                raise DomainError(
                    "fact_cardinality_immutable",
                    "Reconcile claim cardinalities before merging identities",
                )
            evidence = merged_evidence(evidence, record.body.evidence)
        if previous is not None:
            await self.head_in(tx, ctx, previous.reference)
            assert isinstance(previous.body, FactClaimBody)
            if previous.body.cardinality != body.cardinality:
                raise DomainError(
                    "fact_cardinality_immutable",
                    "Reconcile claim cardinalities before merging identities",
                )
            evidence = merged_evidence(previous.body.evidence, evidence)
            body = body.model_copy(
                update={
                    "identity_lineage": tuple(
                        dict.fromkeys((*previous.body.identity_lineage, *body.identity_lineage))
                    )
                }
            )
        conflicting = [
            record
            for record in candidates
            if isinstance(record.body, FactClaimBody)
            and record.body.status in {"active", "conflicted"}
            and conflicts_with(record.body, record.times.validity, body, before.times.validity)
        ]
        conflict = bool(conflicting) or any(
            conflicts_with(other, validity, body, before.times.validity)
            for other, validity in incoming
        )
        body = body.model_copy(
            update={"evidence": evidence, "status": "conflicted" if conflict else "active"}
        )
        command = FactChange(action="classify", reason=reason, refresh_profile=False)
        migrated = await self.revise_in(
            tx,
            ctx,
            body,
            previous,
            report,
            "migrate_scope",
            command,
            apply_requested_validity=False,
            origin=before,
            extra_evidence=tuple(link for record in originals for link in record.evidence)
            + tuple(
                EvidenceLink(
                    reference=record.reference, polarity="boundary", role="identity migration"
                )
                for record in originals
            ),
        )
        changes.append(migrated)
        for record in conflicting:
            assert isinstance(record.body, FactClaimBody)
            if record.body.status != "conflicted":
                changes.append(
                    await self.revise_in(
                        tx,
                        ctx,
                        record.body.model_copy(update={"status": "conflicted"}),
                        record,
                        report,
                        "classify",
                        command,
                        apply_requested_validity=False,
                    )
                )
        for original in originals:
            assert isinstance(original.body, FactClaimBody)
            mapping[original.reference.id] = migrated.reference
            changes.append(
                await self.revise_in(
                    tx,
                    ctx,
                    original.body.model_copy(update={"status": "superseded"}),
                    original,
                    report,
                    "migrate_scope",
                    command,
                    apply_requested_validity=False,
                    extra_evidence=(
                        EvidenceLink(
                            reference=migrated.reference,
                            polarity="boundary",
                            role="identity successor",
                        ),
                    ),
                )
            )
    return tuple({record.reference.id: record for record in changes}.values()), mapping
