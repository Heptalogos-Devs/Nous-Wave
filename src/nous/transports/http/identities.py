"""Identity metadata and lifecycle management use the same domain owner."""

from fastapi import APIRouter, Request
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.identity import (
    BindSubject,
    BoundSubject,
    ChangeNamespace,
    ChangeSubject,
    CreateNamespace,
    CreateSubject,
    IdentityCapabilities,
    IdentityDirectoryQuery,
    IdentityPurpose,
    Namespace,
    NamespacePage,
    NamespacePurgeResult,
    PurgeNamespace,
    ReadSubjectBinding,
    Subject,
    SubjectPage,
)
from nous_sdk.contracts.namespace_archives import (
    MigrateSubject,
    NamespaceArchive,
    NamespaceRestoreResult,
    RestoreNamespace,
    SubjectMigrationResult,
)

from nous.identity.credentials import Principal

from .security import application, authorize, context

router = APIRouter()


@router.post("/namespaces/{namespace_id}/purge")
async def purge_namespace(
    request: Request, namespace_id: str, command: PurgeNamespace
) -> NamespacePurgeResult:
    return await application(request).namespace_data.purge_namespace(
        await manager_context(request, namespace_id), command
    )


async def metadata_context(
    request: Request, namespace_id: str = "instance", purpose: IdentityPurpose = "active"
) -> tuple[Principal, CallContext]:
    actor = await authorize(request, capability="manage" if purpose == "management" else "read")
    if (
        namespace_id != "instance"
        and actor.namespace_ids is not None
        and namespace_id not in actor.namespace_ids
    ):
        raise DomainError("namespace_scope_forbidden", "Credential does not grant this namespace")
    return actor, CallContext(
        caller_id=actor.caller_id, namespace_id=namespace_id, credential_source_ids=actor.source_ids
    )


async def manager_context(
    request: Request, namespace_id: str, subject_id: str | None = None
) -> CallContext:
    actor = await authorize(request, capability="manage")
    actor.require_scope(namespace_id, subject_id)
    if actor.source_ids is not None:
        raise DomainError(
            "identity_management_scope_forbidden",
            "Identity management requires the complete target scope",
        )
    return await context(
        request, namespace_id, subject_id, capability="manage", identity_purpose="management"
    )


@router.post("/namespaces/query")
async def namespace_page(request: Request, command: IdentityDirectoryQuery) -> NamespacePage:
    actor, ctx = await metadata_context(request, purpose=command.purpose)
    return await application(request).identity.namespace_page(
        ctx, command, allowed_namespaces=actor.namespace_ids, allowed_subjects=actor.subject_ids
    )


@router.get("/namespaces")
async def namespaces(
    request: Request, purpose: IdentityPurpose = "active"
) -> tuple[Namespace, ...]:
    actor, ctx = await metadata_context(request, purpose=purpose)
    values: list[Namespace] = []
    command = IdentityDirectoryQuery(purpose=purpose, limit=1000)
    while True:
        page = await application(request).identity.namespace_page(
            ctx, command, allowed_namespaces=actor.namespace_ids, allowed_subjects=actor.subject_ids
        )
        values.extend(page.items)
        if page.next_cursor is None:
            return tuple(values)
        command = command.model_copy(update={"cursor": page.next_cursor})


@router.post("/namespaces")
async def create_namespace(request: Request, command: CreateNamespace) -> Namespace:
    return await application(request).identity.create_namespace(
        await manager_context(request, command.id or "instance"),
        command.name,
        command.operation_id,
        namespace_id=command.id,
    )


@router.get("/namespaces/{namespace_id}")
async def namespace(
    request: Request, namespace_id: str, purpose: IdentityPurpose = "active"
) -> Namespace:
    actor, ctx = await metadata_context(request, namespace_id, purpose)
    return await application(request).identity.namespace(
        ctx, purpose, allowed_subjects=actor.subject_ids
    )


@router.post("/namespaces/{namespace_id}/change")
async def change_namespace(
    request: Request, namespace_id: str, command: ChangeNamespace
) -> Namespace:
    return await application(request).identity.change_namespace(
        await manager_context(request, namespace_id), command
    )


@router.post("/namespaces/{namespace_id}/subjects/query")
async def subject_page(
    request: Request, namespace_id: str, command: IdentityDirectoryQuery
) -> SubjectPage:
    actor, ctx = await metadata_context(request, namespace_id, command.purpose)
    return await application(request).identity.subject_page(
        ctx, command, allowed_subjects=actor.subject_ids
    )


@router.get("/namespaces/{namespace_id}/subjects")
async def subjects(
    request: Request, namespace_id: str, purpose: IdentityPurpose = "active"
) -> tuple[Subject, ...]:
    actor, ctx = await metadata_context(request, namespace_id, purpose)
    values: list[Subject] = []
    command = IdentityDirectoryQuery(purpose=purpose, limit=1000)
    while True:
        page = await application(request).identity.subject_page(
            ctx, command, allowed_subjects=actor.subject_ids
        )
        values.extend(page.items)
        if page.next_cursor is None:
            return tuple(values)
        command = command.model_copy(update={"cursor": page.next_cursor})


@router.post("/namespaces/{namespace_id}/subjects")
async def create_subject(request: Request, namespace_id: str, command: CreateSubject) -> Subject:
    return await application(request).identity.create_subject(
        await manager_context(request, namespace_id),
        command.name,
        command.operation_id,
        capabilities=command.capabilities,
        subject_id=command.id,
    )


@router.get("/namespaces/{namespace_id}/subjects/{subject_id}")
async def subject(
    request: Request, namespace_id: str, subject_id: str, purpose: IdentityPurpose = "active"
) -> Subject:
    return await application(request).identity.subject(
        await context(
            request,
            namespace_id,
            subject_id,
            capability="manage" if purpose == "management" else "read",
            identity_purpose=purpose,
        ),
        purpose,
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/change")
async def change_subject(
    request: Request, namespace_id: str, subject_id: str, command: ChangeSubject
) -> Subject:
    return await application(request).identity.change_subject(
        await manager_context(request, namespace_id, subject_id), command
    )


@router.get("/namespaces/{namespace_id}/capabilities")
async def namespace_capabilities(
    request: Request, namespace_id: str, purpose: IdentityPurpose = "active"
) -> IdentityCapabilities:
    actor, ctx = await metadata_context(request, namespace_id, purpose)
    await application(request).identity.namespace(ctx, purpose, allowed_subjects=actor.subject_ids)
    return await application(request).identity.capabilities(ctx, purpose)


@router.get("/namespaces/{namespace_id}/subjects/{subject_id}/capabilities")
async def subject_capabilities(
    request: Request, namespace_id: str, subject_id: str, purpose: IdentityPurpose = "active"
) -> IdentityCapabilities:
    return await application(request).identity.capabilities(
        await context(
            request,
            namespace_id,
            subject_id,
            capability="manage" if purpose == "management" else "read",
            identity_purpose=purpose,
        ),
        purpose,
    )


@router.post("/namespaces/{namespace_id}/subject-bindings")
async def bind_subject(
    request: Request, namespace_id: str, command: BindSubject, subject_id: str | None = None
) -> BoundSubject:
    return await application(request).identity.bind(
        await manager_context(request, namespace_id, command.subject_id or subject_id), command
    )


@router.post("/namespaces/{namespace_id}/subject-bindings/read")
async def subject_binding(
    request: Request, namespace_id: str, command: ReadSubjectBinding, subject_id: str | None = None
) -> BoundSubject:
    return await application(request).identity.binding(
        await context(
            request,
            namespace_id,
            subject_id,
            capability="manage" if command.purpose == "management" else "read",
            identity_purpose=command.purpose,
        ),
        command.external_id,
        command.purpose,
    )


@router.post("/namespaces/{namespace_id}/archive")
async def export_namespace_archive(request: Request, namespace_id: str) -> NamespaceArchive:
    (await authorize(request, capability="manage")).require_instance_management()
    return await application(request).namespace_archives.export(
        await manager_context(request, namespace_id)
    )


@router.post("/namespaces/{namespace_id}/restore-archive")
async def restore_namespace_archive(
    request: Request, namespace_id: str, command: RestoreNamespace
) -> NamespaceRestoreResult:
    (await authorize(request, capability="manage")).require_instance_management()
    return await application(request).namespace_archives.restore(
        await manager_context(request, namespace_id), command
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/migrate")
async def migrate_subject(
    request: Request, namespace_id: str, subject_id: str, command: MigrateSubject
) -> SubjectMigrationResult:
    actor = await authorize(request, capability="manage")
    actor.require_scope(command.target_namespace_id)
    return await application(request).namespace_archives.migrate_subject(
        await manager_context(request, namespace_id, subject_id), command
    )
