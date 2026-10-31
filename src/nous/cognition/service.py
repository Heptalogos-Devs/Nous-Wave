"""Authority for exact cognitive revisions and their supporting roots."""

from __future__ import annotations

from collections.abc import Awaitable, Callable
from datetime import datetime
from typing import TYPE_CHECKING
from uuid import uuid4

from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    CognitiveTimes,
    DomainError,
    EvidenceLink,
    JournalBody,
    MediaRecord,
    MediaReference,
    MemoryBody,
    RecordPurpose,
    RetentionState,
    RevisionCause,
    TimePoint,
)
from sqlalchemy import select, true, update

from nous.identity.access import require_subject_in
from nous.material.coverage import merge_source_regions
from nous.material.derived import DerivedService
from nous.material.service import (
    MaterialService,
    observations,
    require_observation_in,
    source_permitted,
)
from nous.platform.persistence import Authority, ReadView, WriteTx
from nous.references import allocate_in

from . import (
    access,
    ego_model,
    entity_changes,
    entity_projection,
    erasure,
    evolution,
    formation,
    graph_changes,
    graph_state,
    growth,
    growth_validation,
    known_outputs,
    memory,
    motivation_records,
    retirement,
    social,
)
from . import (
    records as record_directory,
)
from . import (
    retention as retention_owner,
)
from .projection import evidence_in, media_references_in, references_in
from .revision_state import RevisionState
from .schema import revisions
from .semantics import capability_for, validate_domain_in

if TYPE_CHECKING:
    from nous.vector_projection.service import VectorProjectionService

    from .facts import FactService
    from .models import ModelServices


class CognitionService:
    project_record_in = entity_projection.project_record_in

    async def revise_evidence_in(
        self,
        tx: WriteTx,
        ctx: CallContext,
        reference: CognitiveRef,
        *,
        add: tuple[EvidenceLink, ...] = (),
        remove: tuple[EvidenceLink, ...] = (),
        reason: str,
    ) -> CognitiveRecord:
        from .evidence_changes import revise_evidence_in

        return await revise_evidence_in(
            self, tx, ctx, reference, add=add, remove=remove, reason=reason
        )

    require_retention_preconditions_in = retention_owner.require_preconditions_in
    change_retention_in = retention_owner.change_in
    change_retentions_in = retention_owner.change_many_in
    change_retention = retention_owner.change
    change_retentions = retention_owner.change_many
    retention_snapshot = retention_owner.read_snapshot
    decay_retention = retention_owner.decay
    decay_retention_in = retention_owner.decay_in
    support_available_in = access.support_available_in
    erase_history_in = erasure.erase_history_in
    purge_cognition_in = erasure.purge_cognition_in
    search_graph = graph_state.search_graph
    read_graph = graph_state.read_graph
    merge_entities = entity_changes.merge_entities
    split_entity = entity_changes.split_entity
    records_in = record_directory.records_in
    read_records_in = record_directory.read_records_in
    read_records = record_directory.read_records
    record_detail = record_directory.record_detail
    records = record_directory.records
    growth_snapshot_in = growth.growth_snapshot_in
    apply_growth_in = growth.apply_growth_in
    validate_growth_in = growth_validation.validate_growth_in
    retirement_plan_in = retirement.retirement_plan_in
    retire_unsupported_in = retirement.retire_unsupported_in
    link_identity = social.link_identity
    resolve_identity = social.resolve_identity
    change_social_relation = social.change_social_relation
    social_profile = social.social_profile
    propose_identity = social.propose_identity
    unlink_identity = social.unlink_identity
    history = access.history
    replay_record_in = access.replay_record_in
    automatic_in = access.automatic_in
    replay_records_in = access.replay_records_in
    concept_by_alias = access.concept_by_alias
    set_deleted = access.set_deleted
    set_deleted_in = access.set_deleted_in
    graph_snapshot = graph_state.graph_snapshot
    manage_graph = graph_changes.manage_graph
    qualify_known_outputs_in = known_outputs.qualify_known_outputs_in
    adopt_seed = ego_model.adopt_seed
    ego_profile = ego_model.ego_profile
    merge_concepts = evolution.merge_concepts
    merge_concepts_in = evolution.merge_concepts_in
    split_concept = evolution.split_concept
    split_concept_in = evolution.split_concept_in
    resegment_episodes = evolution.resegment_episodes
    resegment_episodes_in = evolution.resegment_episodes_in
    assess_motivation = motivation_records.assess_motivation
    motivation_assessment = motivation_records.read_assessment
    latest_motivation_assessment_in = motivation_records.latest_assessment_in
    motivation_assessments = motivation_records.list_assessments
    purge_motivation_assessments_in = motivation_records.purge_assessments_in
    change_memory_in = memory.change_memory_in
    change_memory = memory.change_memory
    revise_source_times_in = memory.revise_source_times_in
    change_memories = memory.change_memories
    change_memories_in = memory.change_memories_in
    recycle_bin = memory.recycle_bin
    decay_memories = memory.decay_memories
    adopt_tags_in = formation.adopt_tags_in
    formation_catalog_in = formation.formation_catalog_in
    formation_catalog = formation.formation_catalog
    adopt_text_formation_in = formation.adopt_text_formation_in
    adopt_semantic_proposal_in = formation.adopt_semantic_proposal_in

    def __init__(
        self,
        authority: Authority,
        *,
        materials: MaterialService | None = None,
        derivations: DerivedService | None = None,
        models: ModelServices | None = None,
        invalidate: Callable[[], Awaitable[None]] | None = None,
        purge_context_in: Callable[
            [WriteTx, CallContext, tuple[CognitiveRef, ...]], Awaitable[None]
        ]
        | None = None,
    ) -> None:
        self.authority = authority
        self.materials = materials or MaterialService(authority)
        self.derivations = derivations or DerivedService(authority, self.materials)
        self.models = models
        self.invalidate = invalidate
        self.purge_context_in = purge_context_in
        self.projection: VectorProjectionService | None = None
        self.fact_owner: FactService | None = None

    async def source_roots_in(
        self,
        view: ReadView | WriteTx,
        ctx: CallContext,
        *,
        purpose: RecordPurpose = "current",
        include_purged: bool = False,
    ):
        from nous.material.read_facts import source_facts_in

        facts = await source_facts_in(view, ctx)
        if facts is not None:
            return {
                event.id: event.material_id
                for event in facts.observations.values()
                if (purpose == "management" or event.active)
                and (include_purged or not event.purged)
            }
        rows = (
            await view.connection.execute(
                select(observations.c.id, observations.c.material_id).where(
                    observations.c.namespace_id == ctx.namespace_id,
                    observations.c.subject_id == ctx.subject_id,
                    observations.c.active.is_(True) if purpose != "management" else true(),
                    observations.c.record["purged"].as_boolean().is_not(True)
                    if not include_purged
                    else true(),
                )
            )
        ).all()
        return {str(row.id): str(row.material_id) for row in rows}

    async def visible_in(
        self,
        view: ReadView | WriteTx,
        ctx: CallContext,
        record: CognitiveRecord | MediaRecord,
        roots: dict[str, str] | None = None,
        *,
        purpose: RecordPurpose = "exact",
    ) -> bool:
        if record.namespace_id != ctx.namespace_id or record.subject_id != ctx.subject_id:
            return False
        roots = (
            roots if roots is not None else await self.source_roots_in(view, ctx, purpose=purpose)
        )
        if (
            isinstance(record, CognitiveRecord)
            and not record.purged
            and purpose != "management"
            and not await access.support_available_in(self, view, ctx, record, roots)
        ):
            return False
        live = [oid for oid in record.source_roots if oid in roots]
        from nous.material.read_facts import source_facts_in

        facts = await source_facts_in(view, ctx)
        contact_sources = (
            {
                identity: event.contact_source
                for identity in live
                if (event := facts.observations.get(identity)) is not None
                and event.kind == "contact"
                and event.contact_source is not None
            }
            if facts
            else {
                str(row.id): MediaReference.model_validate(row.record["contact_source"])
                for row in (
                    await view.connection.execute(
                        select(observations.c.id, observations.c.record).where(
                            observations.c.namespace_id == ctx.namespace_id,
                            observations.c.subject_id == ctx.subject_id,
                            observations.c.id.in_(live),
                            observations.c.kind == "contact",
                        )
                    )
                ).all()
                if row.record.get("contact_source") is not None
            }
        )
        support = set(record.support_roots or record.source_roots)
        if not support.intersection(roots):
            return False
        for group in ctx.source_groups():
            if record.reference.exact() in group or record.reference.id in group:
                continue
            scoped = ctx.for_source_group(group)
            media_roots: set[str] = set()
            if isinstance(record, CognitiveRecord):
                for media in dict.fromkeys(
                    (*media_references_in(record.body), *contact_sources.values())
                ):
                    try:
                        if media.kind in {"representation", "derived_region"}:
                            info = await self.derivations.source_in(view, scoped, media)
                            media_roots.update(info.roots)
                        else:
                            provenance = await self.materials.provenance_in(
                                view, scoped, media, derivations=self.derivations
                            )
                            media_roots.update(provenance.roots)
                        media_roots.update(
                            oid for oid, source in contact_sources.items() if source == media
                        )
                    except DomainError as error:
                        if error.code not in {
                            "source_unavailable",
                            "region_not_found",
                            "representation_not_found",
                            "derived_region_not_found",
                        }:
                            raise
            if not all(
                oid in media_roots
                or source_permitted(scoped, roots[oid], oid)
                or (
                    bool(record.source_regions.get(oid))
                    and all(
                        region in group
                        or f"source-region:{region}" in group
                        or f"source_region:{region}" in group
                        for region in record.source_regions[oid]
                    )
                )
                for oid in live
            ):
                return False
        return True

    async def get_in(
        self,
        view: ReadView | WriteTx,
        ctx: CallContext,
        reference: CognitiveRef,
        *,
        check_visibility: bool = True,
        purpose: RecordPurpose = "exact",
    ):
        row = (
            await view.connection.execute(
                select(revisions.c.record).where(
                    revisions.c.id == reference.id,
                    revisions.c.revision == reference.revision,
                    revisions.c.kind == reference.kind,
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                )
            )
        ).scalar_one_or_none()
        if row is None:
            raise DomainError("cognition_not_found", "Cognitive revision is unavailable")
        record = CognitiveRecord.model_validate(row)
        if purpose == "current":
            from .lifecycle import selectable

            head = await view.connection.scalar(
                select(revisions.c.revision).where(
                    revisions.c.id == reference.id,
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.head.is_(True),
                )
            )
            if head != reference.revision or not selectable(record):
                raise DomainError(
                    "cognition_inactive", "The revision is not current available cognition"
                )
        if check_visibility and not await self.visible_in(view, ctx, record, purpose=purpose):
            raise DomainError(
                "source_unavailable", "Cognitive sources are unavailable in this scope"
            )
        return record

    async def read_in(
        self,
        view: ReadView | WriteTx,
        ctx: CallContext,
        reference: CognitiveRef,
        *,
        purpose: RecordPurpose = "exact",
    ) -> CognitiveRecord:
        if purpose == "management":
            return await self.get_in(view, ctx, reference, purpose=purpose)
        record = await self.get_in(view, ctx, reference, check_visibility=False, purpose=purpose)
        record = await self.project_record_in(view, ctx, record)
        if not await self.visible_in(view, ctx, record, purpose=purpose):
            raise DomainError(
                "source_unavailable", "Cognitive sources are unavailable in this scope"
            )
        return record

    async def adopt_in(
        self,
        tx: WriteTx,
        ctx: CallContext,
        command: AdoptCognition,
        *,
        state: RevisionState | None = None,
        retiring_entities: frozenset[str] = frozenset(),
        fact_owner: FactService | None = None,
    ):
        await require_subject_in(tx, ctx, capability_for(command.body))
        object_id = command.id or str(uuid4())
        head = (
            await tx.connection.execute(
                select(revisions.c.revision).where(
                    revisions.c.id == object_id,
                    revisions.c.namespace_id == ctx.namespace_id,
                    revisions.c.subject_id == ctx.subject_id,
                    revisions.c.head.is_(True),
                )
            )
        ).scalar_one_or_none()
        if (head or 0) != command.expected_revision:
            raise DomainError(
                "revision_conflict",
                "Cognition was changed since the expected revision",
                expected=command.expected_revision,
                actual=head or 0,
            )
        existing_record: CognitiveRecord | None = None
        if head:
            existing_record = CognitiveRecord.model_validate(
                (
                    await tx.connection.execute(
                        select(revisions.c.record).where(
                            revisions.c.id == object_id, revisions.c.revision == head
                        )
                    )
                ).scalar_one()
            )
            if existing_record.purged:
                raise DomainError(
                    "cognition_purged", "A purged cognitive identity cannot be revived"
                )
            existing_kind = (
                await tx.connection.execute(
                    select(revisions.c.kind).where(
                        revisions.c.id == object_id, revisions.c.revision == head
                    )
                )
            ).scalar_one()
            if existing_kind != command.body.kind:
                raise DomainError(
                    "cognitive_kind_immutable", "A cognitive identity cannot change its domain kind"
                )
        if command.body.kind == "episode" and (
            existing_record is None or command.body != existing_record.body
        ):
            from nous.platform.configuration.namespace_policy import require_feature_in

            await require_feature_in(tx, ctx, "episodes")
        if command.body.kind == "fact_claim":
            owned_content = (
                fact_owner is not None
                and fact_owner.cognition is self
                and fact_owner.authority is self.authority
            )
            unchanged_content_state = (
                existing_record is not None
                and state is not None
                and command.body == existing_record.body
                and command.times == existing_record.times
                and command.producer_call_id == existing_record.producer_call_id
                and all(link in command.evidence for link in existing_record.evidence)
                and all(
                    link in existing_record.evidence or link.polarity == "boundary"
                    for link in command.evidence
                )
            )
            if not owned_content and not unchanged_content_state:
                raise DomainError(
                    "fact_owner_required",
                    "Fact content and applicability changes require the Fact owner",
                )
        roots: set[str] = set()
        support: set[str] = set()
        requirements: list[tuple[str, tuple[str, ...]]] = []
        unchanged_lifecycle = (
            state is not None
            and state.cause.action in {"trash", "restore"}
            and existing_record is not None
            and command.body == existing_record.body
        )
        cutoff = (
            command.body.knowledge_cutoff
            if isinstance(command.body, JournalBody) and not unchanged_lifecycle
            else None
        )
        if cutoff is not None and cutoff > tx.commit_seq:
            raise DomainError("knowledge_cutoff", "Journal perspective is outside recorded history")

        def require_known(sequence: int) -> None:
            if cutoff is not None and sequence > cutoff:
                raise DomainError(
                    "knowledge_cutoff", "Journal cites knowledge after its writing perspective"
                )

        def require_root(observation_id: str, region_ids: tuple[str, ...]) -> None:
            roots.add(observation_id)
            requirements.append((observation_id, region_ids))

        body_evidence = tuple(evidence_in(command.body))
        for evidence in (*command.evidence, *body_evidence):
            if evidence.observation_id:
                observation = await require_observation_in(
                    tx, ctx, evidence.observation_id, region_id=evidence.region_id
                )
                if evidence.polarity != "boundary":
                    require_known(int(observation["commit_seq"]))
                source_ref = MediaReference(
                    kind="source_region" if evidence.region_id else "material",
                    id=evidence.region_id or observation["material_id"],
                    observation_id=evidence.observation_id,
                )
                provenance = await self.materials.provenance_in(
                    tx, ctx, source_ref, derivations=self.derivations
                )
                for root in provenance.roots:
                    require_root(root, provenance.regions.get(root, ()))
                if evidence.polarity == "support":
                    support.update(provenance.roots)
                if isinstance(command.body, MemoryBody) and provenance.roots != (
                    evidence.observation_id,
                ):
                    command = command.model_copy(
                        update={
                            "body": command.body.model_copy(
                                update={
                                    "media": tuple(dict.fromkeys((*command.body.media, source_ref)))
                                }
                            )
                        }
                    )
            if evidence.reference:
                source = await self.read_in(tx, ctx, evidence.reference)
                if evidence.polarity != "boundary":
                    require_known(source.commit_seq)
                if source.purged and evidence.polarity == "support":
                    raise DomainError(
                        "cognition_purged", "A purged revision cannot support new cognitive content"
                    )
                for observation_id in source.source_roots:
                    require_root(observation_id, source.source_regions.get(observation_id, ()))
                if evidence.polarity == "support":
                    support.update(source.support_roots or source.source_roots)
        for media in media_references_in(command.body):
            if media.kind in {"representation", "derived_region"}:
                info = await self.derivations.source_in(tx, ctx, media)
                for observation_id in info.roots:
                    require_root(observation_id, info.regions.get(observation_id, ()))
                support.update(info.roots)
            else:
                provenance = await self.materials.provenance_in(
                    tx, ctx, media, derivations=self.derivations
                )
                for root in provenance.roots:
                    require_root(root, provenance.regions.get(root, ()))
                support.update(provenance.roots)
        for reference in references_in(command.body):
            if reference.id == object_id and reference.revision >= (head or 0) + 1:
                raise DomainError("self_reference", "A revision cannot reference itself")
            source = await self.read_in(tx, ctx, reference)
            for observation_id in source.source_roots:
                require_root(observation_id, source.source_regions.get(observation_id, ()))
            if isinstance(command.body, JournalBody) and reference in command.body.episodes:
                require_known(source.commit_seq)
        if isinstance(command.body, MemoryBody) and command.body.contact_observation_id:
            contact_id = command.body.contact_observation_id
            contact = await tx.connection.scalar(
                select(observations.c.record).where(
                    observations.c.id == contact_id,
                    observations.c.namespace_id == ctx.namespace_id,
                    observations.c.subject_id == ctx.subject_id,
                    observations.c.kind == "contact",
                    observations.c.active.is_(True),
                )
            )
            if (
                contact is None
                or MediaReference.model_validate(contact.get("contact_source"))
                not in command.body.media
            ):
                raise DomainError(
                    "contact_source_mismatch", "Memory must retain its actual encountered selection"
                )
            selected = MediaReference.model_validate(contact["contact_source"])
            require_root(contact_id, (selected.id,) if selected.kind == "source_region" else ())
        await validate_domain_in(self, tx, ctx, command.body, object_id, retiring_entities)
        if not support:
            raise DomainError(
                "grounding_required", "Cognitive adoption requires a live supporting source"
            )
        if command.body.kind == "schema" and not command.body.support:
            raise DomainError("support_required", "Schema requires supporting cognition")
        if command.body.kind == "episode":
            ordinals = [member.ordinal for member in command.body.members]
            if len(ordinals) != len(set(ordinals)):
                raise DomainError("episode_order", "Episode member ordinals must be unique")
        now = ctx.clock
        times = command.times or CognitiveTimes(formation=now, recorded=now)
        # The recorded axis is the authority's real clock, independent of host event time.
        recorded = TimePoint(
            coordinate=int(datetime.fromisoformat(tx.recorded_at).timestamp() * 1000)
        )
        times = times.model_copy(update={"recorded": recorded})
        state = state or RevisionState(
            cause=RevisionCause(
                action="adopt",
                actor=ctx.caller_id,
                reason="cognitive adoption",
                parent=existing_record.reference if existing_record else None,
            ),
            lifecycle=(
                existing_record.lifecycle
                if existing_record and existing_record.lifecycle
                else RetentionState(
                    anchor_unix_ms=existing_record.times.recorded.coordinate
                    if existing_record
                    else recorded.coordinate
                )
            )
            if command.body.kind in {"memory", "association"}
            else None,
            deleted=existing_record.deleted if existing_record else False,
        )
        if (
            command.body.kind in {"memory", "association"}
            and state.lifecycle is None
            and not state.purged
        ):
            from dataclasses import replace

            from .lifecycle import state_for

            state = replace(
                state,
                lifecycle=state_for(existing_record)
                if existing_record is not None
                else RetentionState(anchor_unix_ms=recorded.coordinate),
            )
        result = CognitiveRecord(
            reference=CognitiveRef(kind=command.body.kind, id=object_id, revision=(head or 0) + 1),
            namespace_id=ctx.namespace_id,
            subject_id=ctx.require_subject(),
            body=None if state.purged else command.body,
            evidence=command.evidence,
            source_roots=tuple(sorted(roots)),
            support_roots=tuple(sorted(support)),
            source_regions=merge_source_regions(requirements),
            times=times,
            producer_call_id=command.producer_call_id,
            commit_seq=tx.commit_seq,
            lifecycle=state.lifecycle,
            deleted=state.deleted,
            purged=state.purged,
            cause=state.cause,
        )
        result = result.model_copy(
            update={"addresses": await allocate_in(tx, ctx, result.reference)}
        )
        if head:
            await tx.connection.execute(
                update(revisions)
                .where(revisions.c.id == object_id, revisions.c.head.is_(True))
                .values(head=False)
            )
        await tx.connection.execute(
            revisions.insert().values(
                id=object_id,
                revision=result.reference.revision,
                namespace_id=ctx.namespace_id,
                subject_id=ctx.require_subject(),
                kind=result.reference.kind,
                head=True,
                record=result.model_dump(mode="json"),
                commit_seq=tx.commit_seq,
            )
        )
        if self.projection is not None and not (
            command.body.kind in {"memory", "association"}
            and existing_record is not None
            and command.body == existing_record.body
            and state.cause.action
            in {
                "reinforce",
                "weaken",
                "protect",
                "unprotect",
                "pin",
                "unpin",
                "freeze",
                "forget",
                "restore",
                "trash",
                "purge",
                "decay",
                "remember_forever",
                "allow_decay",
            }
        ):
            await self.projection.changed_in(tx, ctx, result)
        return result

    async def adopt(self, ctx: CallContext, command: AdoptCognition):
        async with self.authority.write(
            ctx,
            command.operation_id,
            operation_kind="cognition.adopt",
            scope=ctx.require_subject(),
            payload=command,
        ) as tx:
            if tx.replayed:
                return await self.replay_record_in(
                    tx, ctx, CognitiveRecord.model_validate(tx.replay_result)
                )
            return tx.result(await self.adopt_in(tx, ctx, command))

    async def get(self, ctx: CallContext, reference: CognitiveRef):
        async with self.authority.read(ctx) as view:
            await require_subject_in(view, ctx)
            return await self.read_in(view, ctx, reference)
