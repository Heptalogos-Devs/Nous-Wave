"""Entity fields retain their own live proof while independent descriptions remain readable."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    CallContext,
    CognitiveRecord,
    EntityBody,
    EvidenceLink,
    MediaReference,
)
from nous_sdk.contracts.fact_body import FactClaimBody
from sqlalchemy import select

from nous.platform.persistence import ReadView, WriteTx

from .lifecycle import selectable
from .projection import evidence_in
from .schema import revisions

if TYPE_CHECKING:
    from .service import CognitionService


async def live_links_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    links: tuple[EvidenceLink, ...],
    roots: dict[str, str],
    *,
    require_current: bool = False,
) -> tuple[EvidenceLink, ...]:
    kept: list[EvidenceLink] = []
    for link in links:
        if link.polarity != "support":
            continue
        live = False
        if link.observation_id in roots:
            assert link.observation_id is not None
            reference = MediaReference(
                kind="source_region" if link.region_id else "material",
                id=link.region_id or roots[link.observation_id],
                observation_id=link.observation_id,
            )
            live = await self.derivations.availability_in(view, ctx, reference) == "available"
        elif link.reference is not None:
            raw = await view.connection.scalar(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.id == link.reference.id,
                    revisions.c.revision == link.reference.revision,
                )
            )
            if raw is not None:
                source = CognitiveRecord.model_validate(raw)
                live = await self.support_available_in(view, ctx, source, roots)
                if live and (require_current or isinstance(source.body, FactClaimBody)):
                    head_raw = await view.connection.scalar(
                        select(revisions.c.record).where(
                            revisions.c.namespace_id == ctx.namespace_id,
                            revisions.c.subject_id == ctx.subject_id,
                            revisions.c.id == source.reference.id,
                            revisions.c.head.is_(True),
                        )
                    )
                    head = (
                        CognitiveRecord.model_validate(head_raw) if head_raw is not None else None
                    )
                    live = (
                        head is not None
                        and head.reference == link.reference
                        and selectable(head)
                        and not (
                            isinstance(head.body, FactClaimBody) and head.body.status != "active"
                        )
                    )
        if live:
            kept.append(link)
    return tuple(kept)


async def links_live_in(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    links: tuple[EvidenceLink, ...],
    roots: dict[str, str],
    *,
    require_all: bool = False,
) -> bool:
    kept = await live_links_in(self, view, ctx, links, roots, require_current=require_all)
    return bool(kept) and (
        not require_all or len(kept) == sum(link.polarity == "support" for link in links)
    )


async def project_record_in(
    self: CognitionService, view: ReadView | WriteTx, ctx: CallContext, record: CognitiveRecord
) -> CognitiveRecord:
    if not isinstance(record.body, EntityBody):
        return record
    body = record.body
    roots = await self.source_roots_in(view, ctx)
    base_links = await live_links_in(self, view, ctx, record.evidence, roots)
    base_live = bool(base_links)
    attributes = tuple(
        [
            attribute
            for attribute in body.attributes
            if await links_live_in(self, view, ctx, attribute.evidence, roots)
        ]
    )
    controls = body.profile_controls
    if controls.aliases is not None and not await links_live_in(
        self, view, ctx, controls.aliases.evidence, roots
    ):
        controls = controls.model_copy(update={"aliases": None})
    if controls.summary is not None and not await links_live_in(
        self, view, ctx, controls.summary.evidence, roots
    ):
        controls = controls.model_copy(update={"summary": None})
    generated = body.generated_profile
    if generated is not None and not await links_live_in(
        self, view, ctx, generated.evidence, roots, require_all=True
    ):
        generated = None
    body = body.model_copy(
        update={
            "description": body.description if base_live else "",
            "aliases": body.aliases if base_live else (),
            "attributes": attributes,
            "profile_controls": controls,
            "generated_profile": generated,
        }
    )
    supported: set[str] = set()
    for link in (*base_links, *tuple(evidence_in(body))):
        if link.polarity != "support":
            continue
        if link.observation_id is not None and link.observation_id in roots:
            supported.add(link.observation_id)
        elif link.reference is not None:
            raw = await view.connection.scalar(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.id == link.reference.id,
                    revisions.c.revision == link.reference.revision,
                )
            )
            if raw is not None:
                source = CognitiveRecord.model_validate(raw)
                supported.update(root for root in source.source_roots if root in roots)
    return record.model_copy(
        update={
            "body": body,
            "evidence": base_links,
            "source_roots": tuple(sorted(supported)),
            "support_roots": tuple(sorted(supported)),
            "source_regions": {
                root: selectors
                for root, selectors in record.source_regions.items()
                if root in supported
            },
        }
    )
