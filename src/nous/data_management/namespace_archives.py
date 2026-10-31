"""Namespace-scoped semantic backup and receiving-Subject migration.

Operational queues, Context work areas and installation credentials are owned by
installation backup; this archive preserves the scoped semantic data explicitly.
"""

from uuid import NAMESPACE_URL, uuid5

import sqlalchemy as sa
from nous_sdk.contracts import BundleRequest, CallContext, DomainError, InstallBundle, SubjectBundle
from nous_sdk.contracts.identity import ChangeNamespace, ChangeSubject, Subject, SubjectBinding
from nous_sdk.contracts.namespace_archives import (
    ArchivedConfiguration,
    ArchivedSubject,
    MigrateSubject,
    NamespaceArchive,
    NamespaceRestoreResult,
    RestoreNamespace,
    SubjectMigrationResult,
)
from pydantic import JsonValue, TypeAdapter

from nous.identity.access import require_namespace_in, require_subject_in
from nous.identity.schema import subject_bindings, subjects
from nous.identity.service import IdentityService, require_management_scope
from nous.platform.configuration.service import ConfigurationService, configuration
from nous.platform.persistence import Authority, ReadView

from .bundles import BundleService


class NamespaceArchiveService:
    def __init__(
        self,
        authority: Authority,
        identity: IdentityService,
        bundles: BundleService,
        configuration_owner: ConfigurationService,
    ):
        self.authority, self.identity, self.bundles, self.configuration = (
            authority,
            identity,
            bundles,
            configuration_owner,
        )

    async def export(self, ctx: CallContext) -> NamespaceArchive:
        require_management_scope(ctx)
        if ctx.subject_id is not None:
            raise DomainError(
                "namespace_scope_required", "Namespace archive requires the complete namespace"
            )
        async with self.authority.read(ctx) as view:
            namespace = await require_namespace_in(view, ctx, purpose="management")
            rows = (
                (
                    await view.connection.execute(
                        sa.select(subjects)
                        .where(subjects.c.namespace_id == ctx.namespace_id)
                        .order_by(subjects.c.id)
                    )
                )
                .mappings()
                .all()
            )
            archived: list[ArchivedSubject] = []
            for row in rows:
                subject_ctx = ctx.model_copy(update={"subject_id": row["id"]})
                subject = Subject.model_validate(
                    await require_subject_in(view, subject_ctx, purpose="management")
                )
                bundle = SubjectBundle.model_validate(
                    await self.bundles.export_in(
                        view, subject_ctx, BundleRequest(), purpose="management"
                    )
                )
                archived.append(ArchivedSubject(subject=subject, bundle=bundle))
            scopes = {
                f"namespace:{ctx.namespace_id}": None,
                **{f"subject:{value.subject.id}": value.subject.id for value in archived},
            }
            configs = (
                (
                    await view.connection.execute(
                        sa.select(configuration).where(configuration.c.scope.in_(scopes))
                    )
                )
                .mappings()
                .all()
            )
            bindings = (
                (
                    await view.connection.execute(
                        sa.select(subject_bindings).where(
                            subject_bindings.c.namespace_id == ctx.namespace_id
                        )
                    )
                )
                .mappings()
                .all()
            )
            return NamespaceArchive(
                namespace=namespace,
                authority_commit_seq=view.commit_seq,
                subjects=tuple(archived),
                bindings=tuple(SubjectBinding.model_validate(row) for row in bindings),
                configuration=tuple(
                    ArchivedConfiguration(
                        scope="subject" if scopes[row["scope"]] else "namespace",
                        subject_id=scopes[row["scope"]],
                        key=row["key"],
                        value=row["value"],
                    )
                    for row in configs
                ),
            )

    async def restore(self, ctx: CallContext, command: RestoreNamespace) -> NamespaceRestoreResult:
        require_management_scope(ctx)
        if ctx.subject_id is not None or ctx.namespace_id == "instance":
            raise DomainError(
                "namespace_scope_required", "Select a new receiving namespace identity"
            )
        receipt = await self.authority.receipt(
            ctx,
            command.operation_id,
            operation_kind="namespace.restore_archive",
            scope=ctx.namespace_id,
            payload=command,
        )
        if receipt is not None:
            return NamespaceRestoreResult.model_validate(receipt.result)
        archive = command.archive
        original_ids = [entry.subject.id for entry in archive.subjects]
        if len(original_ids) != len(set(original_ids)) or any(
            entry.subject.namespace_id != archive.namespace.id
            or entry.bundle.namespace_id != archive.namespace.id
            or entry.bundle.subject_id != entry.subject.id
            for entry in archive.subjects
        ):
            raise DomainError(
                "namespace_archive_scope", "Archived Subjects must belong to the archived namespace"
            )
        for entry in archive.configuration:
            if entry.scope == "subject" and entry.subject_id not in original_ids:
                raise DomainError(
                    "namespace_archive_scope", "Configuration names an unarchived Subject"
                )
            definition = self.configuration.entry(entry.key)
            if entry.scope not in definition.scopes:
                raise DomainError(
                    "configuration_scope_invalid", "Archived configuration has an invalid scope"
                )
            definition.parse(entry.value)
        if (
            archive.namespace.memory_subject_id is not None
            and archive.namespace.memory_subject_id not in original_ids
        ):
            raise DomainError(
                "namespace_archive_scope", "Protocol binding names an unarchived Subject"
            )

        def step(label: str) -> str:
            return str(
                uuid5(
                    NAMESPACE_URL,
                    f"nous:namespace-restore:{ctx.caller_id}:{ctx.namespace_id}:{command.operation_id}:{label}",
                )
            )

        await self.identity.create_namespace(
            ctx,
            command.name or archive.namespace.name,
            step("namespace"),
            namespace_id=ctx.namespace_id,
        )
        # Incomplete imports remain inaccessible; retry continues the same native receipts.
        await self.identity.change_namespace(
            ctx, ChangeNamespace(operation_id=step("disable"), action="disable")
        )
        mapping: dict[str, str] = {}
        installations: dict[str, dict[str, JsonValue]] = {}
        for entry in archive.subjects:
            target_id = step("subject:" + entry.subject.id)
            mapping[entry.subject.id] = target_id
            target_ctx = ctx.model_copy(update={"subject_id": target_id})
            await self.identity.create_subject(
                ctx,
                entry.subject.name,
                step("create:" + entry.subject.id),
                capabilities=("memory",),
                subject_id=target_id,
                purpose="management",
            )
            result = await self.bundles.install(
                target_ctx,
                InstallBundle(
                    operation_id=step("bundle:" + entry.subject.id),
                    bundle=entry.bundle.model_dump(mode="json"),
                    mode="restore",
                ),
                purpose="management",
            )
            installations[entry.subject.id] = TypeAdapter(dict[str, JsonValue]).validate_python(
                result
            )
            await self.identity.change_subject(
                target_ctx,
                ChangeSubject(
                    operation_id=step("capabilities:" + entry.subject.id),
                    action="configure_capabilities",
                    capabilities=entry.subject.capabilities,
                ),
            )
            if archive.namespace.memory_subject_id == entry.subject.id:
                await self.identity.change_namespace(
                    ctx,
                    ChangeNamespace(
                        operation_id=step("memory"),
                        action="bind_memory_subject",
                        subject_id=target_id,
                    ),
                )
            if not entry.subject.enabled:
                await self.identity.change_subject(
                    target_ctx,
                    ChangeSubject(
                        operation_id=step("disable:" + entry.subject.id), action="disable"
                    ),
                )
            if entry.subject.deleted:
                await self.identity.change_subject(
                    target_ctx,
                    ChangeSubject(operation_id=step("trash:" + entry.subject.id), action="trash"),
                )
        for index, entry in enumerate(archive.configuration):
            target_ctx = (
                ctx.model_copy(update={"subject_id": mapping[entry.subject_id]})
                if entry.subject_id
                else ctx
            )
            await self.configuration.set(
                target_ctx, entry.key, entry.value, step(f"configuration:{index}"), entry.scope
            )
        await self.identity.restore_bindings(ctx, archive.bindings, mapping, step("bindings"))
        if archive.namespace.enabled:
            await self.identity.change_namespace(
                ctx, ChangeNamespace(operation_id=step("enable"), action="enable")
            )
        if archive.namespace.deleted:
            await self.identity.change_namespace(
                ctx, ChangeNamespace(operation_id=step("trash"), action="trash")
            )
        result = NamespaceRestoreResult(
            namespace=await self.identity.namespace(ctx, "management"),
            subjects=mapping,
            installations=installations,
        )
        async with self.authority.write(
            ctx,
            command.operation_id,
            operation_kind="namespace.restore_archive",
            scope=ctx.namespace_id,
            payload=command,
        ) as tx:
            return (
                NamespaceRestoreResult.model_validate(tx.replay_result)
                if tx.replayed
                else tx.result(result)
            )

    async def migrate_subject(
        self, ctx: CallContext, command: MigrateSubject
    ) -> SubjectMigrationResult:
        require_management_scope(ctx)
        source = await self.identity.subject(ctx, "management")
        target_ctx = ctx.model_copy(
            update={"namespace_id": command.target_namespace_id, "subject_id": None}
        )
        await self.identity.namespace(target_ctx)
        operation = str(
            uuid5(
                NAMESPACE_URL,
                f"nous:subject-migrate:{ctx.caller_id}:{ctx.namespace_id}:{source.id}:{command.operation_id}",
            )
        )
        target_id = str(uuid5(NAMESPACE_URL, operation + ":subject"))
        receipt = await self.authority.receipt(
            ctx,
            command.operation_id,
            operation_kind="subject.migrate",
            scope=source.id,
            payload=command,
        )
        if receipt is not None:
            return SubjectMigrationResult.model_validate(receipt.result)
        async with self.authority.write(
            ctx,
            operation + ":snapshot",
            operation_kind="subject.migration.snapshot",
            scope=source.id,
            payload=command,
        ) as tx:
            if tx.replayed:
                captured = TypeAdapter(dict[str, JsonValue]).validate_python(tx.replay_result)
            else:
                current = Subject.model_validate(
                    await require_subject_in(tx, ctx, purpose="management")
                )
                captured = {
                    "subject": current.model_dump(mode="json"),
                    "bundle": await self.bundles.export_in(
                        ReadView(tx.connection, tx.commit_seq - 1),
                        ctx,
                        BundleRequest(),
                        purpose="management",
                    ),
                }
                tx.result(captured)
        source = Subject.model_validate(captured["subject"])
        bundle = TypeAdapter(dict[str, JsonValue]).validate_python(captured["bundle"])
        await self.identity.create_subject(
            target_ctx,
            command.name or source.name,
            operation + ":create",
            capabilities=("memory",),
            subject_id=target_id,
        )
        target_ctx = target_ctx.model_copy(update={"subject_id": target_id})
        result = await self.bundles.install(
            target_ctx,
            InstallBundle(operation_id=operation + ":install", bundle=bundle, mode="restore"),
        )
        await self.identity.change_subject(
            target_ctx,
            ChangeSubject(
                operation_id=operation + ":capabilities",
                action="configure_capabilities",
                capabilities=source.capabilities,
            ),
        )
        if not source.enabled:
            await self.identity.change_subject(
                target_ctx, ChangeSubject(operation_id=operation + ":disable", action="disable")
            )
        if source.deleted:
            await self.identity.change_subject(
                target_ctx, ChangeSubject(operation_id=operation + ":trash", action="trash")
            )
        value = SubjectMigrationResult(
            source_subject_id=source.id,
            subject=await self.identity.subject(target_ctx, "management"),
            installation=TypeAdapter(dict[str, JsonValue]).validate_python(result),
        )
        async with self.authority.write(
            ctx,
            command.operation_id,
            operation_kind="subject.migrate",
            scope=source.id,
            payload=command,
        ) as tx:
            return (
                SubjectMigrationResult.model_validate(tx.replay_result)
                if tx.replayed
                else tx.result(value)
            )
