"""Multi-document Ego seeds preserve actual source contacts and explicit revisions."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    DomainError,
    EgoFacetBody,
    EpistemicStance,
    EvidenceLink,
    FreezeCatalog,
    MemoryBody,
)
from nous_sdk.contracts.ego_model import AdoptSeed, SeedResult
from sqlalchemy import select

from nous.identity.access import require_subject_in
from nous.platform.persistence import ReadView, WriteTx

from .growth_sources import from_units_in
from .growth_sources import read_in as read_sources_in
from .lifecycle import selectable
from .schema import revisions

if TYPE_CHECKING:
    from nous.imports.service import ImportService

    from .service import CognitionService


async def replay_seed(
    self: CognitionService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    command: AdoptSeed,
    cached: SeedResult,
) -> SeedResult:
    for entry in command.documents:
        if entry.document.material_id:
            await self.materials.read_in(view, ctx, entry.document.material_id)
        for facet in entry.facets:
            if facet.target:
                await self.get_in(view, ctx, facet.target)
    frozen = FreezeCatalog(
        operation_id=command.operation_id + ":materials",
        inputs=tuple(entry.document for entry in command.documents),
    )
    frozen_receipt = await self.authority.receipt(
        ctx,
        frozen.operation_id,
        operation_kind="imports.freeze",
        scope=ctx.require_subject(),
        payload=frozen,
    )
    if ctx.source_groups() and frozen_receipt is not None:
        effective = (
            await self.materials.restore_input_in(
                view,
                ctx,
                operation_id=frozen.operation_id,
                operation_kind="imports.freeze",
                scope=ctx.require_subject(),
            )
        ).context
    elif ctx.source_groups():
        effective = await self.qualify_known_outputs_in(
            view,
            ctx,
            (*cached.memories, *cached.facets),
            operation_id=command.operation_id,
            operation_kind="self.seed",
            scope=ctx.require_subject(),
        )
    else:
        effective = ctx
    for reference in (*cached.memories, *cached.facets):
        await self.get_in(view, effective, reference)
    return cached


async def adopt_seed(
    self: CognitionService, ctx: CallContext, command: AdoptSeed, *, imports: ImportService
) -> SeedResult:
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx, "ego")
    receipt = await self.authority.receipt(
        ctx,
        command.operation_id,
        operation_kind="self.seed",
        scope=ctx.require_subject(),
        payload=command,
    )
    if receipt is not None:
        async with self.authority.read(ctx) as view:
            return await replay_seed(
                self, view, ctx, command, SeedResult.model_validate(receipt.result)
            )
    frozen = FreezeCatalog(
        operation_id=command.operation_id + ":materials",
        inputs=tuple(entry.document for entry in command.documents),
    )
    catalog = await imports.freeze_catalog(ctx, frozen)
    if catalog.errors or not catalog.units:
        raise DomainError(
            "seed_input_invalid", "Seed documents require supported readable material"
        )
    effective = ctx
    async with self.authority.read(ctx) as view:
        if ctx.source_groups():
            effective = (
                await self.materials.restore_input_in(
                    view,
                    ctx,
                    operation_id=frozen.operation_id,
                    operation_kind="imports.freeze",
                    scope=ctx.require_subject(),
                )
            ).context
        sources = await from_units_in(self, view, effective, catalog.units)
        sources = await read_sources_in(self, view, effective, sources)
    async with self.authority.write(
        ctx,
        command.operation_id,
        operation_kind="self.seed",
        scope=ctx.require_subject(),
        payload=command,
    ) as tx:
        if tx.replayed:
            return await replay_seed(
                self, tx, ctx, command, SeedResult.model_validate(tx.replay_result)
            )
        await require_subject_in(tx, effective, "ego")
        memories: list[CognitiveRef] = []
        facets: list[CognitiveRef] = []
        observation_ids: list[str] = []
        for entry, source_id in zip(command.documents, catalog.input_source_ids, strict=True):
            selected = tuple(
                source
                for unit, source in zip(catalog.units, sources, strict=True)
                if unit.input_source_id == source_id
            )
            evidence = tuple(link for source in selected for link in source.evidence)
            observation_ids.extend(
                link.observation_id for link in evidence if link.observation_id is not None
            )
            stance = EpistemicStance(classification="reported", attribution="creator_document")
            if entry.enter_memory:
                for source in selected:
                    memory = await self.adopt_in(
                        tx,
                        effective,
                        AdoptCognition(
                            body=MemoryBody(
                                text=source.text,
                                media=(source.source,),
                                role="declarative",
                                epistemic=stance,
                            ),
                            evidence=source.evidence,
                            times=source.times,
                        ),
                    )
                    memories.append(memory.reference)
            for facet in entry.facets:
                prior = await self.get_in(tx, effective, facet.target) if facet.target else None
                if prior is not None and not isinstance(prior.body, EgoFacetBody):
                    raise DomainError(
                        "seed_target_invalid", "Seeded facet updates require an EgoFacet"
                    )
                record = await self.adopt_in(
                    tx,
                    effective,
                    AdoptCognition(
                        id=prior.reference.id if prior else None,
                        expected_revision=prior.reference.revision if prior else 0,
                        body=EgoFacetBody(
                            facet=facet.facet,
                            description=facet.description,
                            contexts=facet.contexts,
                            exceptions=facet.exceptions,
                            category=facet.category,
                            epistemic=stance,
                        ),
                        evidence=evidence
                        + (
                            (
                                EvidenceLink(
                                    reference=prior.reference,
                                    polarity="boundary",
                                    role="superseded_seed",
                                ),
                            )
                            if prior
                            else ()
                        ),
                        times=selected[0].times,
                    ),
                )
                facets.append(record.reference)
        return tx.result(
            SeedResult(
                observation_ids=tuple(dict.fromkeys(observation_ids)),
                memories=tuple(memories),
                facets=tuple(facets),
                commit_seq=tx.commit_seq,
            )
        )


async def ego_profile(self: CognitionService, ctx: CallContext) -> tuple[CognitiveRecord, ...]:
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx, "ego")
        rows = (
            await view.connection.execute(
                select(revisions.c.record).where(
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.head.is_(True),
                    revisions.c.kind.in_(("self_facet", "narrative_thread", "domain_knowledge")),
                )
            )
        ).scalars()
        result: list[CognitiveRecord] = []
        for value in rows:
            record = CognitiveRecord.model_validate(value)
            if selectable(record) and await self.visible_in(view, ctx, record):
                result.append(record)
        return tuple(result)
