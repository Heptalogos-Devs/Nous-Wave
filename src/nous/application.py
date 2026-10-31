"""Explicit resource composition; domain rules stay with their owners."""

from collections.abc import Awaitable, Callable

from nous_sdk.amemorix.contracts import AMemorixPolicy
from nous_sdk.contracts import CallContext, CognitiveRef
from nous_sdk.contracts.context import ContextsPolicy
from nous_sdk.contracts.feedback import FeedbackPolicy
from nous_sdk.contracts.maintenance import GrowthPolicy
from nous_sdk.contracts.media_policy import MediaPolicy
from nous_sdk.contracts.vector_projection import VectorProjectionPolicy

from nous.identity.credentials import CredentialsService
from nous.platform.configuration import Settings
from nous.platform.configuration.models import initial_resources
from nous.platform.configuration.service import ConfigurationEntry, ConfigurationService
from nous.platform.execution import DurableExecution, QueueSpec
from nous.platform.models import ModelService
from nous.platform.network.service import NetworkService
from nous.platform.persistence import Authority, BackupService, WriteTx
from nous.platform.prompts import PromptService


class Application:
    def __init__(self, settings: Settings) -> None:
        from nous.cognition.service import CognitionService
        from nous.identity.service import IdentityService
        from nous.material.service import MaterialService
        from nous.retrieval.policy import defaults as retrieval_defaults

        self.settings = settings
        from nous.model_roles import register as register_model_roles
        from nous.platform.models.roles import RoleRegistry

        roles = RoleRegistry()
        register_model_roles(roles)
        self.authority = Authority(settings.database)
        from nous.platform.configuration.namespace_policy import entries as namespace_entries
        from nous.platform.persistence.namespace_limits import NamespaceLimits

        self.namespace_limits = NamespaceLimits(self.authority)
        self.authority.namespace_limits = self.namespace_limits
        self.credentials = CredentialsService(self.authority, settings.token_file)
        self.configuration = ConfigurationService(
            self.authority,
            {
                **namespace_entries(),
                "models": ConfigurationEntry(
                    initial_resources(settings.model, settings.embedding),
                    instance_management=True,
                    validator=roles.validate_resources,
                    schema=roles.resources_schema,
                ),
                "integrations.amemorix": ConfigurationEntry(AMemorixPolicy()),
                "contexts": ConfigurationEntry(ContextsPolicy()),
                "feedback": ConfigurationEntry(FeedbackPolicy()),
                "network": ConfigurationEntry(settings.network, instance_management=True),
                "retrieval": ConfigurationEntry(retrieval_defaults()),
                "vector_projection": ConfigurationEntry(VectorProjectionPolicy()),
                "growth": ConfigurationEntry(GrowthPolicy()),
                "media": ConfigurationEntry(
                    settings.media, instance_management=True, scopes=("instance",)
                ),
            },
        )
        self.prompts = PromptService(self.authority)
        self.network = NetworkService(self.configuration)
        self.models = ModelService(
            self.authority, settings, self.prompts, self.configuration, self.network, roles=roles
        )
        self.identity = IdentityService(self.authority)
        from nous.data_management.namespace_backups import NamespaceBackupService
        from nous.platform.content import ContentStore
        from nous.platform.media import MediaService

        self.content = ContentStore(self.authority, settings.instance / "content")
        self.namespace_backups = NamespaceBackupService(self.authority, self.identity, self.content, settings.instance / "namespace-backups")
        self.media = MediaService(
            self.content, policy=settings.media, policy_resolver=self.media_policy
        )
        self.backups = BackupService(self.authority, self.content)
        self.material = MaterialService(self.authority, content_store=self.content)
        from nous.data_management.resources import ResourceService

        self.resources = ResourceService(self.authority, self.material, self.network)
        from nous.material.derived import DerivedService

        self.derivations = DerivedService(self.authority, self.material)
        self.cognition = CognitionService(
            self.authority, materials=self.material, derivations=self.derivations, models=self.models
        )
        from nous.cognition.person_profiles import PersonProfileService

        self.person_profiles = PersonProfileService(self.authority, self.cognition, self.models)
        from nous.cognition.facts import FactService

        self.facts = FactService(self.authority, self.cognition, self.person_profiles)
        self.cognition.fact_owner = self.facts
        from nous.cognition.corrections import CorrectionService

        self.corrections = CorrectionService(
            self.authority, self.cognition, self.facts, self.models
        )
        from nous.references.service import ReferenceDirectory

        self.references = ReferenceDirectory(
            self.authority, self.cognition, self.material, resources=self.resources
        )
        self.execution = DurableExecution(
            self.authority,
            queues=(QueueSpec("default"), QueueSpec("formation"), QueueSpec("embedding")),
        )
        from nous.cognition.episodes import EpisodeService

        self.episodes = EpisodeService(self.authority, self.cognition, self.models, self.execution)

    async def start(self) -> None:
        from nous.context.service import ContextService
        from nous.data_management.bundles import BundleService
        from nous.data_management.sources import SourceService
        from nous.imports.media import MediaImportService
        from nous.imports.service import ImportService
        from nous.maintenance import MaintenanceService
        from nous.retrieval.service import RetrievalService

        self.bundles = BundleService(
            self.authority, self.cognition, ledger=self.models.ledger, content_store=self.content
        )
        from nous.data_management.namespace_archives import NamespaceArchiveService

        self.namespace_archives = NamespaceArchiveService(
            self.authority, self.identity, self.bundles, self.configuration
        )
        self.media_imports = MediaImportService(
            self.authority,
            self.material,
            self.cognition,
            self.models,
            self.content,
            self.media,
            self.derivations,
        )
        self.imports = ImportService(
            self.authority, self.cognition, self.models, materials=self.material
        )
        self.imports.media_imports = self.media_imports
        from nous.data_management.dataset_imports import DatasetImportService
        self.datasets = DatasetImportService(self)
        from nous.material.description_deliveries import DescriptionDeliveryService
        self.description_deliveries = DescriptionDeliveryService(self.authority, self.cognition, self.configuration)
        self.maintenance = MaintenanceService(
            self.authority,
            self.cognition,
            self.models,
            self.execution,
            configuration=self.configuration,
            imports=self.imports,
        )
        self.imports.on_contact_in = self.maintenance.on_contact_in
        from nous.cognition.seeds import SeedService

        self.seeds = SeedService(self.authority, self.cognition, self.maintenance, self.imports)
        self.retrieval = RetrievalService(
            self.authority,
            self.cognition,
            self.models,
            self.settings.instance / "indexes",
            configuration=self.configuration,
            materials=self.material,
            derivations=self.derivations,
            resources=self.resources,
        )
        self.models.media_resolver = self.media_imports.resolve_embedding_input
        from nous.retrieval.tuning import RetrievalTuningService

        self.tuning = RetrievalTuningService(self)
        self.models.preview_media_resolver = self.media_imports.generation_input
        from nous.vector_projection.service import VectorProjectionService

        self.vector_projection = VectorProjectionService(
            self.authority,
            self.cognition,
            self.models,
            self.configuration,
            materials=self.material,
            derivations=self.derivations,
            invalidate=self.retrieval.invalidate,
        )
        self.cognition.projection = self.vector_projection
        self.imports.projection = self.vector_projection
        self.retrieval.projection = self.vector_projection
        from nous.vector_projection.spaces import VectorSpaceService

        self.vector_spaces = VectorSpaceService(self.authority, self.cognition, self.models,
            self.configuration, self.vector_projection, self.retrieval.invalidate)
        from nous.diagnostics import SubjectDiagnosticsService

        self.diagnostics = SubjectDiagnosticsService(self)
        from nous.feedback.service import FeedbackService

        self.feedback = FeedbackService(self)
        from nous.integrations.amemorix.service import AMemorixService

        self.amemorix = AMemorixService(self)
        from nous.symbols.service import SymbolsService
        self.symbols = SymbolsService(self)
        self.contexts = ContextService(
            self.authority,
            self.cognition,
            retrieval=self.retrieval,
            materials=self.material,
            configuration=self.configuration,
            models=self.models,
            execution=self.execution,
            derivations=self.derivations,
            feedback_owner=self.feedback,
            symbols=self.symbols,
        )
        self.cognition.invalidate = self.retrieval.invalidate
        self.cognition.purge_context_in = self.purge_cognitive_references_in
        self.sources = SourceService(
            self.authority,
            self.cognition,
            purge_in=self.purge_model_inputs_in,
            content_store=self.content,
            invalidate=self.retrieval.invalidate,
            purge_context_in=self.purge_cognitive_references_in,
        )
        from nous.data_management.namespace_data import NamespaceDataService

        self.namespace_data = NamespaceDataService(self.authority, self.identity, self.content, self.execution, self.sources, self.namespace_backups)
        self.execution.register("nous.imports.process", self.imports.run_process, preemptible=True)
        self.execution.register("nous.imports.dataset", self.datasets.run, preemptible=True)
        self.execution.register("nous.imports.temporal_backfill", self.datasets.run_time_backfill, preemptible=True)
        self.execution.register("nous.feedback.evaluate", self.feedback.run, preemptible=True)
        self.execution.register("nous.feedback.contact", self.feedback.capture_context_event, preemptible=True)
        self.execution.register("nous.sources.trash", self.sources.run_trash, preemptible=True)
        self.execution.register("nous.retrieval.tune", self.tuning.run, preemptible=True)
        self.execution.register(
            "nous.vectors.project", self.vector_projection.run, preemptible=True
        )
        self.execution.register(
            "nous.person_profile.refresh", self.person_profiles.run_refresh, preemptible=True
        )
        self.execution.register(
            "nous.episodes.rebuild", self.episodes.run_rebuild, preemptible=True
        )
        self.execution.register("nous.imports.text", self.imports.run_text_import, preemptible=True)
        self.execution.register(
            "nous.imports.form_observation", self.imports.run_form_observation, preemptible=True
        )
        self.execution.register("nous.context.prepare", self.contexts.run_prepare, preemptible=True)
        self.execution.register(
            "nous.context.resident_tick", self.contexts.resident_tick, preemptible=True
        )
        self.execution.register(
            "nous.context.maintain", self.contexts.run_maintenance, preemptible=True
        )
        self.execution.register(
            "nous.maintenance.generate", self.maintenance.run_generate, preemptible=True
        )
        self.execution.register(
            "nous.maintenance.tick", self.maintenance.run_tick, preemptible=True
        )
        await self.content.initialize()
        await self.authority.initialize()
        await self.execution.start()
        await self.maintenance.reconcile_schedules()

    async def close(self) -> None:
        failures: list[BaseException] = []
        closers: list[Callable[[], Awaitable[None]]] = [self.execution.close]
        if hasattr(self, "retrieval"):
            closers.append(self.retrieval.close)
        closers.extend((self.network.close, self.authority.close))
        for close in closers:
            try:
                await close()
            except BaseException as error:
                failures.append(error)
        if failures:
            raise BaseExceptionGroup("Application resource shutdown failed", failures)

    async def purge_model_inputs_in(self, tx: WriteTx, material_ids: tuple[str, ...]) -> None:
        await self.description_deliveries.purge_in(tx, material_ids=material_ids)
        await self.vector_projection.purge_materials_in(tx, material_ids)
        await self.models.ledger.purge_materials_in(tx, material_ids)
        await self.derivations.purge_materials_in(tx, material_ids)
        await self.maintenance.purge_materials_in(tx, material_ids)
        await self.corrections.purge_materials_in(tx, material_ids)
        await self.resources.purge_materials_in(tx, material_ids)
        await self.symbols.purge_references_in(tx, tx.ctx, material_ids)
        await self.tuning.purge_in(tx, material_ids=material_ids)
        await self.cognition.purge_motivation_assessments_in(tx, material_ids=material_ids)

    async def purge_cognitive_references_in(
        self, tx: WriteTx, ctx: CallContext, references: tuple[CognitiveRef, ...]
    ) -> None:
        await self.contexts.purge_references_in(tx, ctx, references)
        await self.description_deliveries.purge_in(tx, references=references)
        await self.corrections.purge_references_in(tx, ctx, references)
        await self.tuning.purge_in(tx, references=references)
        await self.feedback.purge_references_in(tx, ctx, references)
        await self.symbols.purge_references_in(tx, ctx, (reference.id for reference in references))
        await self.cognition.purge_motivation_assessments_in(tx, references=references)

    async def media_policy(self) -> MediaPolicy:
        effective = await self.configuration.effective_instance("media")
        return MediaPolicy.model_validate(effective["value"])
