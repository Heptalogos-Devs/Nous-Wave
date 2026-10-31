"""Namespace/Subject lifecycle and host bindings share one Authority owner."""

import asyncio
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from uuid import uuid4

import sqlalchemy as sa
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.identity import (
    BindSubject,
    BoundSubject,
    ChangeNamespace,
    ChangeSubject,
    IdentityCapabilities,
    IdentityDirectoryQuery,
    IdentityPurpose,
    Namespace,
    NamespacePage,
    Subject,
    SubjectBinding,
    SubjectPage,
)
from sqlalchemy.sql.elements import ColumnElement

from nous.platform.persistence import Authority, WriteTx

from .access import normalise_capabilities, require_namespace_in, require_subject_in
from .schema import namespaces, retired, subject_bindings, subjects

SUPPORTED_CAPABILITIES = ("memory", "ego", "social", "motivation")


def _configured(values: tuple[str, ...]) -> list[str]:
    result = normalise_capabilities(values)
    unknown = set(result) - set(SUPPORTED_CAPABILITIES)
    if unknown:
        raise DomainError(
            "capability_unsupported", "Unknown Subject capabilities", capabilities=sorted(unknown)
        )
    return result


def require_management_scope(ctx: CallContext) -> None:
    if ctx.source_groups():
        raise DomainError(
            "identity_management_scope_forbidden",
            "Identity management requires the complete target scope",
        )


def _states(table: sa.Table, command: IdentityDirectoryQuery) -> ColumnElement[bool]:
    if command.purpose == "active":
        return sa.and_(table.c.enabled.is_(True), table.c.deleted.is_(False))
    clauses: list[ColumnElement[bool]] = []
    for state in command.states:
        clauses.append(
            table.c.deleted.is_(True)
            if state == "trashed"
            else sa.and_(table.c.deleted.is_(False), table.c.enabled.is_(state == "enabled"))
        )
    return sa.or_(*clauses) if clauses else sa.true()


class IdentityService:
    def __init__(self, authority: Authority) -> None:
        self.authority = authority
        self._namespace_locks: dict[str, asyncio.Lock] = {}

    @asynccontextmanager
    async def namespace_lock(self, namespace_id: str) -> AsyncGenerator[None]:
        async with self._namespace_locks.setdefault(namespace_id, asyncio.Lock()):
            yield

    async def begin_namespace_purge(
        self, ctx: CallContext, operation_id: str, expected_revision: int | None
    ) -> Namespace:
        require_management_scope(ctx)
        if ctx.subject_id is not None:
            raise DomainError(
                "namespace_scope_forbidden", "Namespace purge requires the complete Namespace"
            )
        async with self.authority.write(
            ctx,
            operation_id,
            operation_kind="namespace.purge.begin",
            scope=ctx.namespace_id,
            payload={"expected_revision": expected_revision},
        ) as tx:
            if tx.replayed:
                return Namespace.model_validate(tx.replay_result)
            current = await require_namespace_in(tx, ctx, purpose="management")
            if not current.deleted:
                raise DomainError(
                    "namespace_not_trashed",
                    "Move the Namespace to the recycle bin before permanent removal",
                )
            if (
                expected_revision is not None
                and current.revision != expected_revision
                and not current.purging
            ):
                raise DomainError(
                    "identity_revision_conflict",
                    "Namespace revision changed",
                    actual_revision=current.revision,
                )
            value = current.model_copy(
                update={
                    "purging": True,
                    "enabled": False,
                    "revision": current.revision + 1,
                    "commit_seq": tx.commit_seq,
                    "updated_at": tx.recorded_at,
                }
            )
            await tx.connection.execute(
                namespaces.update()
                .where(namespaces.c.id == ctx.namespace_id)
                .values(**value.model_dump())
            )
            return tx.result(value)

    async def retire_namespace_in(self, tx: WriteTx, ctx: CallContext) -> None:
        require_management_scope(ctx)
        current = await require_namespace_in(tx, ctx, purpose="management")
        if not current.deleted or not current.purging or ctx.subject_id is not None:
            raise DomainError(
                "namespace_not_trashed", "Namespace permanent removal has not been admitted"
            )
        identities = (
            (
                await tx.connection.execute(
                    sa.select(subjects.c.id).where(subjects.c.namespace_id == ctx.namespace_id)
                )
            )
            .scalars()
            .all()
        )
        for kind, identifier in (
            ("namespace", ctx.namespace_id),
            *(("subject", str(value)) for value in identities),
        ):
            await tx.connection.execute(
                retired.insert().values(
                    kind=kind,
                    id=identifier,
                    namespace_id=ctx.namespace_id,
                    purged_at=tx.recorded_at,
                    commit_seq=tx.commit_seq,
                )
            )
        await tx.connection.execute(
            subject_bindings.delete().where(subject_bindings.c.namespace_id == ctx.namespace_id)
        )
        await tx.connection.execute(
            subjects.delete().where(subjects.c.namespace_id == ctx.namespace_id)
        )
        await tx.connection.execute(namespaces.delete().where(namespaces.c.id == ctx.namespace_id))

    async def require_fresh_identity_in(self, tx: WriteTx, kind: str, identifier: str) -> None:
        if await tx.connection.scalar(
            sa.select(retired.c.id).where(retired.c.kind == kind, retired.c.id == identifier)
        ):
            raise DomainError(
                "identity_purged",
                "A permanently removed identity cannot be reused",
                kind=kind,
                id=identifier,
            )

    async def restore_directory_in(
        self,
        tx: WriteTx,
        namespace: Namespace,
        restored_subjects: tuple[Subject, ...],
        bindings: tuple[SubjectBinding, ...],
    ) -> None:
        """Accept a fresh, fully scoped identity directory inside its restore transaction."""
        require_management_scope(tx.ctx)
        if (
            tx.ctx.subject_id is not None
            or namespace.id != tx.ctx.namespace_id
            or namespace.id == "instance"
        ):
            raise DomainError(
                "namespace_backup_scope_forbidden",
                "Identity restore requires its complete receiving namespace",
            )
        if await tx.connection.scalar(
            sa.select(namespaces.c.id).where(namespaces.c.id == namespace.id)
        ):
            raise DomainError(
                "namespace_conflict", "Namespace restore requires a new target namespace"
            )
        await self.require_fresh_identity_in(tx, "namespace", namespace.id)
        identities = {subject.id for subject in restored_subjects}
        if len(identities) != len(restored_subjects) or any(
            subject.namespace_id != namespace.id for subject in restored_subjects
        ):
            raise DomainError(
                "namespace_backup_integrity", "Restored Subjects have inconsistent identity scope"
            )
        if (
            namespace.memory_subject_id is not None
            and namespace.memory_subject_id not in identities
        ):
            raise DomainError(
                "namespace_backup_integrity",
                "Restored protocol binding names an unretained Subject",
            )
        for subject in restored_subjects:
            await self.require_fresh_identity_in(tx, "subject", subject.id)
            _configured(subject.capabilities)
            if subject.id == namespace.memory_subject_id and set(subject.capabilities) != {
                "memory"
            }:
                raise DomainError(
                    "memory_subject_required", "Protocol binding must remain memory-only"
                )
        if any(
            binding.namespace_id != namespace.id or binding.subject_id not in identities
            for binding in bindings
        ):
            raise DomainError(
                "namespace_backup_integrity", "Restored host binding names an unretained Subject"
            )
        await tx.connection.execute(namespaces.insert().values(**namespace.model_dump()))
        for subject in restored_subjects:
            await tx.connection.execute(subjects.insert().values(**subject.model_dump()))
        for binding in bindings:
            await tx.connection.execute(subject_bindings.insert().values(**binding.model_dump()))

    async def create_namespace(
        self, ctx: CallContext, name: str, operation_id: str, namespace_id: str | None = None
    ) -> Namespace:
        require_management_scope(ctx)
        async with self.authority.write(
            ctx,
            operation_id,
            operation_kind="namespace.create",
            scope=ctx.namespace_id,
            payload={"name": name, **({"namespace_id": namespace_id} if namespace_id else {})},
        ) as tx:
            if tx.replayed:
                return Namespace.model_validate(tx.replay_result)
            return tx.result(await self.create_namespace_in(tx, ctx, name, namespace_id))

    async def create_namespace_in(
        self, tx: WriteTx, ctx: CallContext, name: str, namespace_id: str | None = None
    ) -> Namespace:
        require_management_scope(ctx)
        if tx.authority is not self.authority or tx.replayed:
            raise ValueError("Identity creation requires its owning live transaction")
        identifier = namespace_id or (
            ctx.namespace_id if ctx.namespace_id != "instance" else str(uuid4())
        )
        await self.require_fresh_identity_in(tx, "namespace", identifier)
        if await tx.connection.scalar(
            sa.select(namespaces.c.id).where(namespaces.c.id == identifier)
        ):
            raise DomainError("namespace_conflict", "Namespace identity already exists")
        value = Namespace(
            id=identifier,
            name=name,
            commit_seq=tx.commit_seq,
            created_at=tx.recorded_at,
            updated_at=tx.recorded_at,
        )
        await tx.connection.execute(namespaces.insert().values(**value.model_dump()))
        return value

    async def create_subject(
        self,
        ctx: CallContext,
        name: str,
        operation_id: str,
        capabilities: tuple[str, ...] = ("memory",),
        subject_id: str | None = None,
        *,
        purpose: IdentityPurpose = "active",
    ) -> Subject:
        require_management_scope(ctx)
        async with self.authority.write(
            ctx,
            operation_id,
            operation_kind="subject.create",
            scope=ctx.namespace_id,
            payload={"name": name, "capabilities": capabilities, "subject_id": subject_id},
        ) as tx:
            await require_namespace_in(tx, ctx, purpose=purpose)
            if tx.replayed:
                raw = tx.replay_result
                if not isinstance(raw, dict):
                    raise ValueError("Subject creation receipt must be an object")
                replayed = Subject.model_validate(raw)
                return replayed.model_copy(
                    update={"capabilities": tuple(normalise_capabilities(replayed.capabilities))}
                )
            return tx.result(
                await self.create_subject_in(
                    tx, ctx, name, capabilities, subject_id, purpose=purpose
                )
            )

    async def create_subject_in(
        self,
        tx: WriteTx,
        ctx: CallContext,
        name: str,
        capabilities: tuple[str, ...] = ("memory",),
        subject_id: str | None = None,
        *,
        purpose: IdentityPurpose = "active",
    ) -> Subject:
        require_management_scope(ctx)
        if tx.authority is not self.authority or tx.replayed:
            raise ValueError("Identity creation requires its owning live transaction")
        enabled = _configured(capabilities)
        await require_namespace_in(tx, ctx, purpose=purpose)
        identifier = subject_id or str(uuid4())
        await self.require_fresh_identity_in(tx, "subject", identifier)
        if await tx.connection.scalar(sa.select(subjects.c.id).where(subjects.c.id == identifier)):
            raise DomainError("subject_conflict", "Subject identity already exists")
        value = Subject(
            id=identifier,
            namespace_id=ctx.namespace_id,
            name=name,
            capabilities=tuple(enabled),
            commit_seq=tx.commit_seq,
            created_at=tx.recorded_at,
            updated_at=tx.recorded_at,
        )
        await tx.connection.execute(subjects.insert().values(**value.model_dump()))
        return value

    async def namespace(
        self,
        ctx: CallContext,
        purpose: IdentityPurpose = "active",
        *,
        allowed_subjects: tuple[str, ...] | None = None,
    ) -> Namespace:
        if purpose == "management":
            require_management_scope(ctx)
        async with self.authority.read(ctx) as view:
            if allowed_subjects is not None and not await view.connection.scalar(
                sa.select(subjects.c.id)
                .where(
                    subjects.c.namespace_id == ctx.namespace_id, subjects.c.id.in_(allowed_subjects)
                )
                .limit(1)
            ):
                raise DomainError(
                    "namespace_scope_forbidden", "Credential has no Subject in this namespace"
                )
            return await require_namespace_in(view, ctx, purpose=purpose)

    async def subject(self, ctx: CallContext, purpose: IdentityPurpose = "active") -> Subject:
        if purpose == "management":
            require_management_scope(ctx)
        async with self.authority.read(ctx) as view:
            return Subject.model_validate(await require_subject_in(view, ctx, purpose=purpose))

    async def namespace_for_subject(self, subject_id: str) -> str:
        async with self.authority.read() as view:
            value = await view.connection.scalar(
                sa.select(subjects.c.namespace_id).where(subjects.c.id == subject_id)
            )
        if value is None:
            raise DomainError("subject_not_found", "Subject does not exist")
        return str(value)

    async def namespace_page(
        self,
        ctx: CallContext,
        command: IdentityDirectoryQuery,
        *,
        allowed_namespaces: tuple[str, ...] | None = None,
        allowed_subjects: tuple[str, ...] | None = None,
    ) -> NamespacePage:
        if command.purpose == "management":
            require_management_scope(ctx)
        statement = sa.select(namespaces).where(_states(namespaces, command))
        if allowed_namespaces is not None:
            statement = statement.where(namespaces.c.id.in_(allowed_namespaces))
        if allowed_subjects is not None:
            statement = statement.where(
                namespaces.c.id.in_(
                    sa.select(subjects.c.namespace_id).where(subjects.c.id.in_(allowed_subjects))
                )
            )
        if command.cursor:
            statement = statement.where(namespaces.c.id > command.cursor)
        async with self.authority.read(ctx) as view:
            rows = (
                (
                    await view.connection.execute(
                        statement.order_by(namespaces.c.id).limit(command.limit + 1)
                    )
                )
                .mappings()
                .all()
            )
        values = tuple(Namespace.model_validate(row) for row in rows[: command.limit])
        return NamespacePage(
            items=values, next_cursor=values[-1].id if len(rows) > command.limit else None
        )

    async def subject_page(
        self,
        ctx: CallContext,
        command: IdentityDirectoryQuery,
        *,
        allowed_subjects: tuple[str, ...] | None = None,
    ) -> SubjectPage:
        if command.purpose == "management":
            require_management_scope(ctx)
        async with self.authority.read(ctx) as view:
            await require_namespace_in(view, ctx, purpose=command.purpose)
            statement = sa.select(subjects).where(
                subjects.c.namespace_id == ctx.namespace_id, _states(subjects, command)
            )
            if allowed_subjects is not None:
                statement = statement.where(subjects.c.id.in_(allowed_subjects))
            if command.cursor:
                statement = statement.where(subjects.c.id > command.cursor)
            rows = (
                (
                    await view.connection.execute(
                        statement.order_by(subjects.c.id).limit(command.limit + 1)
                    )
                )
                .mappings()
                .all()
            )
        values = tuple(
            Subject.model_validate(
                {**dict(row), "capabilities": normalise_capabilities(row["capabilities"])}
            )
            for row in rows[: command.limit]
        )
        return SubjectPage(
            items=values, next_cursor=values[-1].id if len(rows) > command.limit else None
        )

    async def list_subjects(self, ctx: CallContext):
        values: list[Subject] = []
        command = IdentityDirectoryQuery(limit=1000)
        while True:
            page = await self.subject_page(ctx, command)
            values.extend(page.items)
            if page.next_cursor is None:
                return values
            command = command.model_copy(update={"cursor": page.next_cursor})

    async def capabilities(
        self, ctx: CallContext, purpose: IdentityPurpose = "active"
    ) -> IdentityCapabilities:
        if purpose == "management":
            require_management_scope(ctx)
        async with self.authority.read(ctx) as view:
            namespace = await require_namespace_in(view, ctx, purpose=purpose)
            subject = (
                Subject.model_validate(await require_subject_in(view, ctx, purpose=purpose))
                if ctx.subject_id
                else None
            )
        configured = subject.capabilities if subject else SUPPORTED_CAPABILITIES
        enabled = (
            configured
            if namespace.enabled
            and not namespace.deleted
            and (subject is None or subject.enabled and not subject.deleted)
            else ()
        )
        return IdentityCapabilities(
            namespace_id=ctx.namespace_id,
            subject_id=ctx.subject_id,
            supported=SUPPORTED_CAPABILITIES,
            configured=configured,
            enabled=enabled,
        )

    async def _change(
        self, ctx: CallContext, command: ChangeNamespace | ChangeSubject
    ) -> Namespace | Subject:
        require_management_scope(ctx)
        table = subjects if isinstance(command, ChangeSubject) else namespaces
        identifier = (
            ctx.require_subject() if isinstance(command, ChangeSubject) else ctx.namespace_id
        )
        async with self.authority.write(
            ctx,
            command.operation_id,
            operation_kind="subject.change"
            if isinstance(command, ChangeSubject)
            else "namespace.change",
            scope=identifier,
            payload=command,
        ) as tx:
            if tx.replayed:
                return (
                    Subject.model_validate(tx.replay_result)
                    if isinstance(command, ChangeSubject)
                    else Namespace.model_validate(tx.replay_result)
                )
            current = (
                Subject.model_validate(await require_subject_in(tx, ctx, purpose="management"))
                if isinstance(command, ChangeSubject)
                else await require_namespace_in(tx, ctx, purpose="management")
            )
            parent = await require_namespace_in(tx, ctx, purpose="management")
            if parent.purging:
                raise DomainError("namespace_purging", "Namespace permanent removal is in progress")
            if (
                command.expected_revision is not None
                and current.revision != command.expected_revision
            ):
                raise DomainError(
                    "identity_revision_conflict",
                    "Identity revision changed",
                    actual_revision=current.revision,
                )
            if current.deleted and command.action != "restore":
                raise DomainError("identity_deleted", "Restore this identity before changing it")
            changes: dict[str, object] = {
                "revision": current.revision + 1,
                "commit_seq": tx.commit_seq,
                "updated_at": tx.recorded_at,
            }
            if command.action == "rename":
                if not command.name or not command.name.strip():
                    raise DomainError("identity_name_required", "Supply a nonempty identity name")
                changes["name"] = command.name
            elif command.action in {"enable", "disable"}:
                changes["enabled"] = command.action == "enable"
            elif command.action in {"trash", "restore"}:
                changes["deleted"] = command.action == "trash"
            elif isinstance(command, ChangeNamespace) and command.action == "bind_memory_subject":
                if command.subject_id is None:
                    raise DomainError("subject_required", "Select the memory-only Subject")
                target = Subject.model_validate(
                    await require_subject_in(
                        tx,
                        ctx.model_copy(update={"subject_id": command.subject_id}),
                        purpose="management",
                    )
                )
                if target.deleted or set(target.capabilities) != {"memory"}:
                    raise DomainError(
                        "memory_subject_required",
                        "Binding requires a retained memory-only Subject in this namespace",
                    )
                changes["memory_subject_id"] = target.id
            elif isinstance(command, ChangeSubject):
                if command.capabilities is None:
                    raise DomainError("capabilities_required", "Supply the enabled capability set")
                changes["capabilities"] = tuple(_configured(command.capabilities))
                namespace = await require_namespace_in(tx, ctx, purpose="management")
                if namespace.memory_subject_id == identifier and set(command.capabilities) != {
                    "memory"
                }:
                    raise DomainError(
                        "memory_subject_required",
                        "The namespace protocol binding must remain memory-only",
                    )
            value = type(current).model_validate({**current.model_dump(), **changes})
            await tx.connection.execute(
                table.update().where(table.c.id == identifier).values(**changes)
            )
            return tx.result(value)

    async def change_namespace(self, ctx: CallContext, command: ChangeNamespace) -> Namespace:
        return Namespace.model_validate(await self._change(ctx, command))

    async def change_subject(self, ctx: CallContext, command: ChangeSubject) -> Subject:
        return Subject.model_validate(await self._change(ctx, command))

    async def bind_memory_subject_in(
        self, tx: WriteTx, ctx: CallContext, subject_id: str
    ) -> Namespace:
        require_management_scope(ctx)
        namespace = await require_namespace_in(tx, ctx, purpose="management")
        target = Subject.model_validate(
            await require_subject_in(
                tx, ctx.model_copy(update={"subject_id": subject_id}), purpose="management"
            )
        )
        if target.deleted or set(target.capabilities) != {"memory"}:
            raise DomainError(
                "memory_subject_required",
                "Binding requires a retained memory-only Subject in this namespace",
            )
        value = namespace.model_copy(
            update={
                "memory_subject_id": target.id,
                "revision": namespace.revision + 1,
                "commit_seq": tx.commit_seq,
                "updated_at": tx.recorded_at,
            }
        )
        await tx.connection.execute(
            namespaces.update().where(namespaces.c.id == namespace.id).values(**value.model_dump())
        )
        return value

    async def memory_subject(
        self, ctx: CallContext, purpose: IdentityPurpose = "active"
    ) -> Subject:
        namespace = await self.namespace(ctx, purpose)
        if namespace.memory_subject_id is None:
            raise DomainError(
                "memory_subject_unconfigured", "The namespace has no trusted memory Subject binding"
            )
        if ctx.subject_id is not None and ctx.subject_id != namespace.memory_subject_id:
            raise DomainError(
                "subject_binding_scope_mismatch", "Protocol binding names another Subject"
            )
        subject = await self.subject(
            ctx.model_copy(update={"subject_id": namespace.memory_subject_id}), purpose
        )
        if set(subject.capabilities) != {"memory"}:
            raise DomainError("memory_subject_required", "Protocol binding must remain memory-only")
        return subject

    async def binding(
        self, ctx: CallContext, external_id: str, purpose: IdentityPurpose = "active"
    ) -> BoundSubject:
        if purpose == "management":
            require_management_scope(ctx)
        async with self.authority.read(ctx) as view:
            row = (
                (
                    await view.connection.execute(
                        sa.select(subject_bindings).where(
                            subject_bindings.c.namespace_id == ctx.namespace_id,
                            subject_bindings.c.caller_id == ctx.caller_id,
                            subject_bindings.c.external_id == external_id,
                        )
                    )
                )
                .mappings()
                .first()
            )
            if row is None:
                raise DomainError(
                    "subject_binding_not_found", "Host Subject binding does not exist"
                )
            binding = SubjectBinding.model_validate(row)
            if ctx.subject_id and ctx.subject_id != binding.subject_id:
                raise DomainError(
                    "subject_binding_scope_mismatch", "Binding belongs to another Subject"
                )
            subject = Subject.model_validate(
                await require_subject_in(
                    view, ctx.model_copy(update={"subject_id": binding.subject_id}), purpose=purpose
                )
            )
            return BoundSubject(binding=binding, subject=subject)

    async def bind(self, ctx: CallContext, command: BindSubject) -> BoundSubject:
        require_management_scope(ctx)
        async with self.authority.write(
            ctx,
            command.operation_id,
            operation_kind="subject.bind",
            scope=ctx.namespace_id,
            payload=command,
        ) as tx:
            await require_namespace_in(tx, ctx)
            row = (
                (
                    await tx.connection.execute(
                        sa.select(subject_bindings).where(
                            subject_bindings.c.namespace_id == ctx.namespace_id,
                            subject_bindings.c.caller_id == ctx.caller_id,
                            subject_bindings.c.external_id == command.external_id,
                        )
                    )
                )
                .mappings()
                .first()
            )
            if row is not None:
                binding = SubjectBinding.model_validate(row)
                if ctx.subject_id and ctx.subject_id != binding.subject_id:
                    raise DomainError(
                        "subject_binding_scope_mismatch", "Binding belongs to another Subject"
                    )
                if command.subject_id and command.subject_id != binding.subject_id:
                    raise DomainError(
                        "subject_binding_conflict",
                        "Stable host identity is already bound to another Subject",
                    )
                subject = Subject.model_validate(
                    await require_subject_in(
                        tx, ctx.model_copy(update={"subject_id": binding.subject_id})
                    )
                )
                result = BoundSubject(binding=binding, subject=subject)
                return result if tx.replayed else tx.result(result)
            if tx.replayed:
                raise DomainError(
                    "subject_binding_unavailable", "The original binding is unavailable"
                )
            identifier = command.subject_id or ctx.subject_id or str(uuid4())
            if command.subject_id or ctx.subject_id:
                subject = Subject.model_validate(
                    await require_subject_in(tx, ctx.model_copy(update={"subject_id": identifier}))
                )
            else:
                subject = Subject(
                    id=identifier,
                    namespace_id=ctx.namespace_id,
                    name=command.name,
                    capabilities=tuple(_configured(command.capabilities)),
                    commit_seq=tx.commit_seq,
                    created_at=tx.recorded_at,
                    updated_at=tx.recorded_at,
                )
                await tx.connection.execute(subjects.insert().values(**subject.model_dump()))
            binding = SubjectBinding(
                namespace_id=ctx.namespace_id,
                caller_id=ctx.caller_id,
                external_id=command.external_id,
                subject_id=identifier,
                commit_seq=tx.commit_seq,
            )
            await tx.connection.execute(subject_bindings.insert().values(**binding.model_dump()))
            return tx.result(BoundSubject(binding=binding, subject=subject))

    async def restore_bindings(
        self,
        ctx: CallContext,
        bindings: tuple[SubjectBinding, ...],
        mapping: dict[str, str],
        operation_id: str,
    ) -> tuple[SubjectBinding, ...]:
        """Restore captured host identities only onto validated receiving Subjects."""
        require_management_scope(ctx)
        if ctx.subject_id is not None:
            raise DomainError(
                "namespace_scope_required", "Binding restore requires the complete namespace"
            )
        async with self.authority.write(
            ctx,
            operation_id,
            operation_kind="subject.bindings.restore",
            scope=ctx.namespace_id,
            payload={"bindings": bindings, "mapping": mapping},
        ) as tx:
            if tx.replayed:
                from pydantic import TypeAdapter

                return TypeAdapter(tuple[SubjectBinding, ...]).validate_python(tx.replay_result)
            await require_namespace_in(tx, ctx, purpose="management")
            restored: list[SubjectBinding] = []
            for binding in bindings:
                if binding.subject_id not in mapping:
                    raise DomainError(
                        "subject_binding_scope_mismatch",
                        "Archived binding names an unarchived Subject",
                    )
                target = mapping[binding.subject_id]
                await require_subject_in(
                    tx, ctx.model_copy(update={"subject_id": target}), purpose="management"
                )
                value = binding.model_copy(
                    update={
                        "namespace_id": ctx.namespace_id,
                        "subject_id": target,
                        "commit_seq": tx.commit_seq,
                    }
                )
                await tx.connection.execute(subject_bindings.insert().values(**value.model_dump()))
                restored.append(value)
            return tx.result(tuple(restored))
