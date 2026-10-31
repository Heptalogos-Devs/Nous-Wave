"""Person selection and supported profile evidence belong to the profile domain."""

from __future__ import annotations

import unicodedata
from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, CognitiveRecord, DomainError, EntityBody
from nous_sdk.contracts.fact_body import FactClaimBody, PersonFactScope
from nous_sdk.contracts.facts import FactQuery
from nous_sdk.contracts.person_profiles import (
    PersonProfileEvidenceItem,
    PersonProfilesQuery,
    PersonProfilesResult,
    PersonProfileView,
)
from nous_sdk.contracts.records import RecordsQuery

from nous.material.service import require_observation_in

from .fact_reads import query_in as fact_query_in

if TYPE_CHECKING:
    from .person_profiles import PersonProfileService

_UNAVAILABLE = {"source_unavailable", "cognition_not_found", "region_not_found"}


async def people(self: PersonProfileService, ctx: CallContext) -> tuple[CognitiveRecord, ...]:
    records: list[CognitiveRecord] = []
    async with self.authority.read(ctx) as view:
        from nous.platform.configuration.namespace_policy import require_feature_in

        await require_feature_in(view, ctx, "person_profiles")
        command = RecordsQuery(kinds=("entity",), limit=1000)
        while True:
            page = await self.cognition.records_in(view, ctx, command)
            records.extend(
                item.record
                for item in page.items
                if isinstance(item.record.body, EntityBody)
                and item.record.body.entity_type == "person"
            )
            if page.next_cursor is None:
                return tuple(sorted(records, key=lambda record: record.reference.id))
            command = command.model_copy(update={"cursor": page.next_cursor})


async def select(
    self: PersonProfileService, ctx: CallContext, command: PersonProfilesQuery
) -> CognitiveRecord:
    if command.reference is not None:
        async with self.authority.read(ctx) as view:
            from nous.platform.configuration.namespace_policy import require_feature_in

            await require_feature_in(view, ctx, "person_profiles")
            return await self.person_in(view, ctx, command.reference)
    if not command.identifier and not command.name:
        raise DomainError("person_required", "Select a Person reference, identifier or exact name")
    found: list[CognitiveRecord] = []
    for record in await people(self, ctx):
        assert isinstance(record.body, EntityBody)
        body = record.body
        identifiers = (
            set(body.identifiers.values())
            if command.platform is None
            else {
                value
                for key, value in body.identifiers.items()
                if key.startswith(command.platform + ":")
            }
        )
        matched = (
            command.identifier
            in {
                record.reference.id,
                record.reference.exact(),
                *identifiers,
                *(
                    identity.account_id
                    for identity in body.platform_identities
                    if command.platform is None or identity.platform == command.platform
                ),
            }
            if command.identifier
            else command.name.casefold().strip()
            in {name.casefold().strip() for name in (body.label, *body.effective_aliases)}
        )
        if matched:
            found.append(record)
    if len(found) != 1:
        raise DomainError(
            "person_not_found" if not found else "person_ambiguous",
            "Person selector has no unique authorized match",
        )
    return found[0]


def grams(value: str) -> set[str]:
    normalized = unicodedata.normalize("NFKC", value).casefold()
    token = "".join(char for char in normalized if char.isalnum())
    return {token[index : index + 2] for index in range(len(token) - 1)} - {
        "用户",
        "这个",
        "那个",
        "自己",
        "喜欢",
    }


async def read(
    self: PersonProfileService,
    ctx: CallContext,
    record: CognitiveRecord,
    command: PersonProfilesQuery,
) -> PersonProfileView:
    accepted: list[PersonProfileEvidenceItem] = []
    uncertain: list[PersonProfileEvidenceItem] = []
    async with self.authority.read(ctx) as view:
        from nous.platform.configuration.namespace_policy import require_feature_in

        await require_feature_in(view, ctx, "person_profiles")
        record = await self.person_in(view, ctx, record.reference)
        assert isinstance(record.body, EntityBody)
        if record.body.description:
            accepted.append(
                PersonProfileEvidenceItem(
                    key=f"{record.reference.exact()}#/body/description",
                    kind="entity_description",
                    content=record.body.description,
                    reference=record.reference,
                    evidence=record.evidence,
                )
            )
        for index, attribute in enumerate(record.body.attributes):
            try:
                for link in attribute.evidence:
                    if link.reference is not None:
                        await self.cognition.get_in(view, ctx, link.reference)
                    elif link.observation_id is not None:
                        await require_observation_in(
                            view, ctx, link.observation_id, region_id=link.region_id
                        )
            except DomainError as error:
                if error.code not in _UNAVAILABLE:
                    raise
                continue
            item = PersonProfileEvidenceItem(
                key=f"{record.reference.exact()}#/body/attributes/{index}",
                kind="entity_attribute",
                content=f"{attribute.predicate}: {attribute.value}",
                reference=record.reference,
                attribute_pointer=f"/body/attributes/{index}",
                evidence=attribute.evidence,
                epistemic=attribute.epistemic,
                validity=attribute.validity,
            )
            confirmed = (
                attribute.epistemic.classification in {"observed", "reported"}
                and not any(link.polarity == "counterexample" for link in attribute.evidence)
                and (attribute.validity is None or attribute.validity.contains(ctx.clock))
            )
            (accepted if confirmed else uncertain).append(item)
        claims = await fact_query_in(
            self.cognition,
            view,
            ctx,
            FactQuery(
                scope=PersonFactScope(person=record.reference),
                statuses=("active", "conflicted"),
                limit=max(1, command.evidence_limit),
            ),
        )
        for claim in claims:
            body = claim.body
            assert isinstance(body, FactClaimBody)
            item = PersonProfileEvidenceItem(
                key=claim.reference.exact(),
                kind="fact_claim",
                reference=claim.reference,
                content=f"{body.fact_key} ({body.polarity}): {body.value_text}",
                evidence=(*claim.evidence, *(entry.link for entry in body.evidence)),
                status=body.status,
                stability=body.stability,
                authority=body.authority,
                confidence=body.confidence,
                validity=claim.times.validity,
            )
            confirmed = (
                body.status == "active"
                and body.stability == "stable"
                and not any(entry.link.polarity == "counterexample" for entry in body.evidence)
                and (claim.times.validity is None or claim.times.validity.contains(ctx.clock))
            )
            (accepted if confirmed else uncertain).append(item)
        if command.context_text:
            context = grams(command.context_text)
            uncertain.sort(key=lambda item: (-len(context & grams(item.content)), item.key))
        return PersonProfileView(
            record=record,
            evidence=tuple(accepted[: command.evidence_limit]),
            uncertain=tuple(uncertain[: command.evidence_limit]),
            evidence_count=len(accepted),
            uncertain_count=len(uncertain),
            watermark=view.commit_seq,
        )


async def query(
    self: PersonProfileService, ctx: CallContext, command: PersonProfilesQuery
) -> PersonProfilesResult:
    selected = (
        (await select(self, ctx, command),)
        if command.reference is not None or command.identifier or command.name
        else (await people(self, ctx))[: command.limit]
    )
    return PersonProfilesResult(
        items=tuple([await read(self, ctx, record, command) for record in selected])
    )
