"""Project native Person profile views into the public A-Memorix response shape."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.amemorix.contracts import ProfileInput
from nous_sdk.contracts import CallContext, CognitiveRecord, EntityBody
from nous_sdk.contracts.person_profiles import PersonProfileEvidenceItem, PersonProfilesQuery

if TYPE_CHECKING:
    from .service import AMemorixService


def person_id(record: CognitiveRecord) -> str:
    assert isinstance(record.body, EntityBody)
    return record.body.identifiers.get("maibot:person") or next(
        (
            identity.account_id
            for identity in record.body.platform_identities
            if identity.platform == "maibot"
        ),
        record.reference.id,
    )


async def people(self: AMemorixService, ctx: CallContext) -> tuple[CognitiveRecord, ...]:
    return tuple(
        sorted(
            await self.app.person_profiles.people(ctx),
            key=lambda record: (person_id(record), record.reference.id),
        )
    )


async def select_person(
    self: AMemorixService, ctx: CallContext, command: ProfileInput
) -> CognitiveRecord:
    return await self.app.person_profiles.select(
        ctx,
        PersonProfilesQuery(
            identifier=command.person_id, platform="maibot", name=command.person_name
        ),
    )


def summary(record: CognitiveRecord) -> dict[str, object]:
    assert isinstance(record.body, EntityBody)
    body, point = record.body, record.times.recorded
    override = body.profile_controls.summary
    return {
        "person_id": person_id(record),
        "person_name": body.label,
        "profile_version": record.reference.revision,
        "profile_text": body.profile_text,
        "auto_profile_text": body.generated_profile.summary if body.generated_profile else "",
        "entity_description": body.description,
        "manual_override_text": override.text if override else None,
        "updated_at": point.coordinate / 1000 if point.domain == "unix_ms" else None,
        "profile_source": "nous_entity",
        "has_manual_override": override is not None,
        "manual_override": override.text if override else None,
        "reference": record.reference,
    }


def evidence_item(item: PersonProfileEvidenceItem) -> dict[str, object]:
    metadata = (
        {"epistemic": item.epistemic, "validity": item.validity}
        if item.kind == "entity_attribute"
        else {
            "status": item.status,
            "stability": item.stability,
            "authority": item.authority,
            "confidence": item.confidence,
            "validity": item.validity,
        }
    )
    return {
        "evidence_key": item.key,
        "hash": item.key,
        "evidence_type": item.kind,
        "type": item.kind,
        "deletable": item.kind != "entity_description",
        "correction_mode": "withdraw_attribute_support"
        if item.kind == "entity_attribute"
        else "retract_fact_claim"
        if item.kind == "fact_claim"
        else None,
        "content": item.content,
        "reference": item.reference,
        "attribute_pointer": item.attribute_pointer,
        "evidence": item.evidence,
        "metadata": metadata,
    }


async def read_profile(
    self: AMemorixService, ctx: CallContext, record: CognitiveRecord, command: ProfileInput
) -> dict[str, object]:
    value = await self.app.person_profiles.read(
        ctx,
        record,
        PersonProfilesQuery(
            reference=record.reference,
            evidence_limit=command.limit,
            context_text=command.context_text,
        ),
    )
    assert isinstance(value.record.body, EntityBody)
    return {
        **summary(value.record),
        "success": True,
        "summary": value.record.body.profile_text,
        "traits": [item.content for item in value.evidence],
        "evidence": [evidence_item(item) for item in value.evidence],
        "uncertain": [evidence_item(item) for item in value.uncertain],
        "uncertain_fact_count": value.uncertain_count,
        "evidence_count": value.evidence_count,
        "watermark": value.watermark,
    }


async def aliases(
    self: AMemorixService, ctx: CallContext, record: CognitiveRecord
) -> dict[str, object]:
    async with self.app.authority.read(ctx) as view:
        record = await self.app.person_profiles.person_in(view, ctx, record.reference)
    assert isinstance(record.body, EntityBody)
    control = record.body.profile_controls.aliases
    return {
        "success": True,
        "person_id": person_id(record),
        "primary_name": record.body.label,
        "effective_aliases": record.body.effective_aliases,
        "derived_aliases": tuple(
            dict.fromkeys(
                (
                    *record.body.aliases,
                    *(
                        record.body.generated_profile.aliases
                        if record.body.generated_profile
                        else ()
                    ),
                )
            )
        ),
        "manual_aliases": control.aliases if control else (),
        "has_override": control is not None,
        "has_manual_override": control is not None,
        "reference": record.reference,
    }
