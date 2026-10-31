"""Retire unsupported payloads through actual evidence, preserving independent proof."""

from __future__ import annotations

from collections import deque
from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, CognitiveRecord, CognitiveRef, MemoryBody
from sqlalchemy import select, update

from nous.material.service import observations
from nous.platform.persistence import ReadView, WriteTx

from .entities import contains_record, redact_attributes_in
from .projection import evidence_in, media_references_in
from .schema import revisions, vectors

if TYPE_CHECKING:
    from .service import CognitionService


async def support_snapshot_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    excluded_objects: frozenset[str] = frozenset(),
    excluded_sources: frozenset[str] = frozenset(),
) -> tuple[set[str], dict[str, CognitiveRecord], set[str]]:
    sources = {
        str(row.id)
        for row in (
            await view.connection.execute(
                select(observations.c.id, observations.c.record).where(
                    observations.c.namespace_id == ctx.namespace_id,
                    observations.c.subject_id == ctx.subject_id,
                )
            )
        ).all()
        if not row.record.get("purged") and str(row.id) not in excluded_sources
    }
    values = (
        await view.connection.execute(
            select(revisions.c.record).where(
                revisions.c.namespace_id == ctx.namespace_id,
                revisions.c.subject_id == ctx.subject_id,
            )
        )
    ).scalars()
    records = {
        record.reference.exact(): record
        for record in (CognitiveRecord.model_validate(value) for value in values)
    }
    invalid_media: set[str] = set()
    for key, record in records.items():
        if isinstance(record.body, MemoryBody):
            encounter = record.body.contact_observation_id
            if isinstance(encounter, str) and encounter not in sources:
                invalid_media.add(key)
        for reference in media_references_in(record.body):
            availability = await self.derivations.availability_in(
                view,
                ctx,
                reference,
                purged_observations=excluded_sources,
                purged_cognition=excluded_objects,
            )
            if availability in {"purged", "missing"}:
                invalid_media.add(key)
                break
    supported: set[str] = set()
    dependants: dict[str, set[str]] = {}
    for key, record in records.items():
        if record.purged or record.reference.id in excluded_objects or key in invalid_media:
            continue
        links = tuple(evidence_in(record))
        if tuple(media_references_in(record.body)) and set(record.support_roots).intersection(
            sources
        ):
            supported.add(key)
        for link in links:
            if link.polarity != "support":
                continue
            if link.observation_id in sources:
                supported.add(key)
            if link.reference:
                dependants.setdefault(link.reference.exact(), set()).add(key)
    # Reachability computes the least supported set, including imported cyclic graphs.
    pending = deque(supported)
    while pending:
        for key in dependants.get(pending.popleft(), ()):
            if key not in supported and key not in invalid_media:
                supported.add(key)
                pending.append(key)

    return sources, records, supported


async def retirement_plan_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    object_ids: frozenset[str],
    observation_ids: frozenset[str],
) -> tuple[CognitiveRecord, ...]:
    _, records, supported = await support_snapshot_in(self, view, ctx, object_ids, observation_ids)
    heads: dict[str, CognitiveRecord] = {}
    for record in records.values():
        previous = heads.get(record.reference.id)
        if previous is None or previous.reference.revision < record.reference.revision:
            heads[record.reference.id] = record
    return tuple(
        record
        for record in heads.values()
        if not record.purged
        and (record.reference.id in object_ids or record.reference.exact() not in supported)
    )


async def retire_unsupported_in(
    self: CognitionService, tx: WriteTx, ctx: CallContext
) -> tuple[CognitiveRef, ...]:
    sources, records, supported = await support_snapshot_in(self, tx, ctx)
    retired: list[CognitiveRef] = []
    for record in records.values():
        if record.purged or record.reference.exact() in supported:
            continue
        tombstone = record.model_copy(
            update={
                "body": None,
                "deleted": True,
                "purged": True,
                "evidence": tuple(
                    link.model_copy(update={"role": "purged"}) for link in record.evidence
                ),
            }
        )
        await tx.connection.execute(
            update(revisions)
            .where(
                revisions.c.id == record.reference.id,
                revisions.c.revision == record.reference.revision,
            )
            .values(record=tombstone.model_dump(mode="json"))
        )
        await tx.connection.execute(
            vectors.delete().where(vectors.c.reference == record.reference.exact())
        )
        retired.append(record.reference)
    for record in records.values():
        if record.reference.exact() in supported and await redact_attributes_in(
            tx, record, sources
        ):
            retired.append(record.reference)
    if retired:
        keys = {reference.exact() for reference in retired}
        await self.authority.redact_receipts_in(tx, lambda value: contains_record(value, keys))
    return tuple(retired)
