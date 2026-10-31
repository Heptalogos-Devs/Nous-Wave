"""Typed identity client operations."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from uuid import uuid4

from pydantic import JsonValue, TypeAdapter

from ..contracts.credentials import (
    CreateCredential,
    CredentialEvent,
    CredentialRecord,
    IssuedCredential,
    PrincipalRecord,
    RotateCredential,
)
from ..contracts.diagnostics import SubjectDiagnostics
from ..contracts.identity import (
    BindSubject,
    BoundSubject,
    ChangeNamespace,
    ChangeSubject,
    IdentityCapabilities,
    IdentityDirectoryQuery,
    IdentityPurpose,
    NamespacePage,
    NamespacePurgeResult,
    PurgeNamespace,
    ReadSubjectBinding,
    SubjectPage,
)
from ..contracts.namespace_archives import (
    MigrateSubject,
    NamespaceArchive,
    NamespaceRestoreResult,
    RestoreNamespace,
    SubjectMigrationResult,
)
from ..operations import encoded, segment
from ..types import (
    Namespace,
    Subject,
)
from .backups import NamespaceBackupClient
from .requests import operation_identity, operation_values, required_text


class IdentityClient(NamespaceBackupClient):
    async def purge_namespace(
        self, namespace_id: str, command: PurgeNamespace
    ) -> NamespacePurgeResult:
        return NamespacePurgeResult.model_validate(
            await self.request(
                "POST", f"/api/namespaces/{segment(namespace_id)}/purge", body=encoded(command)
            )
        )

    async def export_namespace_archive(self, namespace_id: str) -> NamespaceArchive:
        return NamespaceArchive.model_validate(
            await self.request("POST", f"/api/namespaces/{segment(namespace_id)}/archive")
        )

    async def restore_namespace_archive(
        self, namespace_id: str, command: RestoreNamespace
    ) -> NamespaceRestoreResult:
        return NamespaceRestoreResult.model_validate(
            await self.request(
                "POST",
                f"/api/namespaces/{segment(namespace_id)}/restore-archive",
                body=encoded(command),
            )
        )

    async def migrate_subject(
        self, namespace_id: str, subject_id: str, command: MigrateSubject
    ) -> SubjectMigrationResult:
        return SubjectMigrationResult.model_validate(
            await self.request(
                "POST",
                f"/api/namespaces/{segment(namespace_id)}/subjects/{segment(subject_id)}/migrate",
                body=encoded(command),
            )
        )

    async def create_namespace(
        self, name: str, *, namespace_id: str | None = None, operation_id: str | None = None
    ) -> Namespace:
        return Namespace.model_validate(
            await self.request(
                "POST",
                "/api/namespaces",
                body={
                    "name": name,
                    "id": namespace_id,
                    "operation_id": operation_id or str(uuid4()),
                },
            )
        )

    async def namespaces(self, *, purpose: IdentityPurpose = "active") -> tuple[Namespace, ...]:
        return TypeAdapter(tuple[Namespace, ...]).validate_python(
            await self.request("GET", "/api/namespaces", params={"purpose": purpose})
        )

    async def namespace(
        self, namespace_id: str, *, purpose: IdentityPurpose = "active"
    ) -> Namespace:
        return Namespace.model_validate(
            await self.request(
                "GET", f"/api/namespaces/{segment(namespace_id)}", params={"purpose": purpose}
            )
        )

    async def namespace_page(self, command: IdentityDirectoryQuery | None = None) -> NamespacePage:
        return NamespacePage.model_validate(
            await self.request(
                "POST", "/api/namespaces/query", body=encoded(command or IdentityDirectoryQuery())
            )
        )

    async def subject_page(
        self, namespace_id: str, command: IdentityDirectoryQuery | None = None
    ) -> SubjectPage:
        return SubjectPage.model_validate(
            await self.request(
                "POST",
                f"/api/namespaces/{segment(namespace_id)}/subjects/query",
                body=encoded(command or IdentityDirectoryQuery()),
            )
        )

    async def change_namespace(self, namespace_id: str, command: ChangeNamespace) -> Namespace:
        return Namespace.model_validate(
            await self.request(
                "POST", f"/api/namespaces/{segment(namespace_id)}/change", body=encoded(command)
            )
        )

    async def change_subject(
        self, namespace_id: str, subject_id: str, command: ChangeSubject
    ) -> Subject:
        return Subject.model_validate(
            await self.request(
                "POST",
                self.subject_path(namespace_id, subject_id) + "/change",
                body=encoded(command),
            )
        )

    async def identity_capabilities(
        self,
        namespace_id: str,
        subject_id: str | None = None,
        *,
        purpose: IdentityPurpose = "active",
    ) -> IdentityCapabilities:
        path = (
            self.subject_path(namespace_id, subject_id)
            if subject_id
            else f"/api/namespaces/{segment(namespace_id)}"
        )
        return IdentityCapabilities.model_validate(
            await self.request("GET", path + "/capabilities", params={"purpose": purpose})
        )

    async def bind_subject(self, namespace_id: str, command: BindSubject) -> BoundSubject:
        return BoundSubject.model_validate(
            await self.request(
                "POST",
                f"/api/namespaces/{segment(namespace_id)}/subject-bindings",
                body=encoded(command),
                params={"subject_id": self.subject_id},
            )
        )

    async def subject_binding(
        self, namespace_id: str, external_id: str, *, purpose: IdentityPurpose = "active"
    ) -> BoundSubject:
        return BoundSubject.model_validate(
            await self.request(
                "POST",
                f"/api/namespaces/{segment(namespace_id)}/subject-bindings/read",
                body=encoded(ReadSubjectBinding(external_id=external_id, purpose=purpose)),
                params={"subject_id": self.subject_id},
            )
        )

    async def create_subject(
        self,
        namespace_id: str,
        *,
        name: str = "default",
        capabilities: Sequence[str] = ("memory",),
        operation_id: str | None = None,
        subject_id: str | None = None,
    ) -> Subject:
        return Subject.model_validate(
            await self.request(
                "POST",
                f"/api/namespaces/{segment(namespace_id)}/subjects",
                body={
                    "name": name,
                    "capabilities": list(capabilities),
                    "id": subject_id,
                    "operation_id": operation_id or str(uuid4()),
                },
            )
        )

    async def subjects(
        self, namespace_id: str, *, purpose: IdentityPurpose = "active"
    ) -> tuple[Subject, ...]:
        return TypeAdapter(tuple[Subject, ...]).validate_python(
            await self.request(
                "GET",
                f"/api/namespaces/{segment(namespace_id)}/subjects",
                params={"purpose": purpose},
            )
        )

    async def subject(self, subject_id: str, *, purpose: IdentityPurpose = "active") -> Subject:
        return Subject.model_validate(
            await self.request(
                "GET", f"/api/subjects/{segment(subject_id)}", params={"purpose": purpose}
            )
        )

    async def subject_diagnostics(self, subject_id: str) -> SubjectDiagnostics:
        return SubjectDiagnostics.model_validate(
            await self.request("GET", f"/api/subjects/{segment(subject_id)}/diagnostics")
        )

    async def whoami(self) -> PrincipalRecord:
        return PrincipalRecord.model_validate(await self.request("GET", "/api/credentials/whoami"))

    async def create_credential(self, command: CreateCredential) -> IssuedCredential:
        return IssuedCredential.model_validate(
            await self.request("POST", "/api/credentials", body=encoded(command))
        )

    async def credentials(self, *, include_revoked: bool = True) -> tuple[CredentialRecord, ...]:
        return TypeAdapter(tuple[CredentialRecord, ...]).validate_python(
            await self.request(
                "GET", "/api/credentials", params={"include_revoked": include_revoked}
            )
        )

    async def credential(self, credential_id: str) -> CredentialRecord:
        return CredentialRecord.model_validate(
            await self.request("GET", f"/api/credentials/{segment(credential_id)}")
        )

    async def credential_history(self, credential_id: str) -> tuple[CredentialEvent, ...]:
        return TypeAdapter(tuple[CredentialEvent, ...]).validate_python(
            await self.request("GET", f"/api/credentials/{segment(credential_id)}/history")
        )

    async def rotate_credential(
        self, credential_id: str, command: RotateCredential
    ) -> IssuedCredential:
        return IssuedCredential.model_validate(
            await self.request(
                "POST", f"/api/credentials/{segment(credential_id)}/rotate", body=encoded(command)
            )
        )

    async def revoke_credential(
        self,
        credential_id: str,
        *,
        expected_revision: int | None = None,
        reason: str = "",
        operation_id: str | None = None,
    ) -> CredentialRecord:
        return CredentialRecord.model_validate(
            await self.request(
                "DELETE",
                f"/api/credentials/{segment(credential_id)}",
                params={
                    "operation_id": operation_id or str(uuid4()),
                    "expected_revision": expected_revision,
                    "reason": reason,
                },
            )
        )

    async def identity(self, request: Mapping[str, JsonValue]) -> JsonValue:
        values = operation_values(request)
        action = request.get("action")
        if action == "whoami":
            return encoded(await self.whoami())
        if action == "list":
            rows = await self.credentials(
                include_revoked=TypeAdapter(bool).validate_python(
                    values.pop("include_revoked", True)
                )
            )
            after = values.pop("after", None)
            if after is not None:
                offset = next(
                    (index + 1 for index, row in enumerate(rows) if row.id == after), None
                )
                if offset is None:
                    raise ValueError("Credential cursor is unavailable")
                rows = rows[offset:]
            limit = TypeAdapter(int).validate_python(values.pop("limit", 100))
            if not 1 <= limit <= 1000:
                raise ValueError("limit must be between 1 and 1000")
            return [encoded(row) for row in rows[:limit]]
        if action == "issue":
            return encoded(await self.create_credential(CreateCredential.model_validate(values)))
        if action == "credential":
            raise ValueError(
                "Credential plaintext is delivered only when issuing or rotating the key"
            )
        credential_id = required_text(values, "id")
        if action == "read":
            return encoded(await self.credential(credential_id))
        if action == "history":
            return [encoded(row) for row in await self.credential_history(credential_id)]
        if action == "rotate":
            return encoded(
                await self.rotate_credential(credential_id, RotateCredential.model_validate(values))
            )
        if action == "revoke":
            return encoded(
                await self.revoke_credential(
                    credential_id,
                    expected_revision=TypeAdapter(int | None).validate_python(
                        values.pop("expected_revision", None)
                    ),
                    reason=TypeAdapter(str).validate_python(values.pop("reason", "")),
                    operation_id=operation_identity(values),
                )
            )
        if action == "credential":
            raise ValueError(
                "Credential plaintext is delivered only when issuing or rotating the key"
            )
        raise ValueError("Unknown identity action")
