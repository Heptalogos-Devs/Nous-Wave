"""Namespace and Subject lifecycle, enabled domains and stable host bindings."""

from typing import Literal

from pydantic import Field

from .common import Contract, Operation

IdentityPurpose = Literal["active", "management"]


class CreateNamespace(Operation):
    name: str = Field(min_length=1, max_length=256)
    id: str | None = Field(default=None, min_length=1, max_length=256)


class CreateSubject(Operation):
    name: str = Field(default="default", min_length=1, max_length=256)
    capabilities: tuple[str, ...] = ("memory",)
    id: str | None = Field(default=None, min_length=1, max_length=256)


class Namespace(Contract):
    id: str
    name: str
    enabled: bool = True
    deleted: bool = False
    revision: int = 1
    commit_seq: int = 0
    created_at: str | None = None
    updated_at: str | None = None
    memory_subject_id: str | None = None
    purging: bool = False


class Subject(Contract):
    id: str
    namespace_id: str
    name: str
    capabilities: tuple[str, ...]
    commit_seq: int
    enabled: bool = True
    deleted: bool = False
    revision: int = 1
    created_at: str | None = None
    updated_at: str | None = None


class IdentityDirectoryQuery(Contract):
    purpose: IdentityPurpose = "active"
    limit: int = Field(default=100, ge=1, le=1000)
    cursor: str | None = None
    states: tuple[Literal["enabled", "disabled", "trashed"], ...] = ()


class NamespacePage(Contract):
    items: tuple[Namespace, ...]
    next_cursor: str | None = None


class SubjectPage(Contract):
    items: tuple[Subject, ...]
    next_cursor: str | None = None


class ChangeNamespace(Operation):
    action: Literal["rename", "enable", "disable", "trash", "restore", "bind_memory_subject"]
    expected_revision: int | None = Field(default=None, ge=1)
    name: str | None = Field(default=None, min_length=1, max_length=256)
    subject_id: str | None = None


class ChangeSubject(Operation):
    action: Literal["rename", "configure_capabilities", "enable", "disable", "trash", "restore"]
    expected_revision: int | None = Field(default=None, ge=1)
    name: str | None = Field(default=None, min_length=1, max_length=256)
    capabilities: tuple[str, ...] | None = None


class IdentityCapabilities(Contract):
    namespace_id: str
    subject_id: str | None = None
    supported: tuple[str, ...]
    configured: tuple[str, ...]
    enabled: tuple[str, ...]


class BindSubject(Operation):
    external_id: str = Field(min_length=1, max_length=512)
    subject_id: str | None = None
    name: str = Field(default="default", min_length=1, max_length=256)
    capabilities: tuple[str, ...] = ("memory",)


class SubjectBinding(Contract):
    namespace_id: str
    caller_id: str
    external_id: str
    subject_id: str
    commit_seq: int


class ReadSubjectBinding(Contract):
    external_id: str = Field(min_length=1, max_length=512)
    purpose: IdentityPurpose = "active"


class BoundSubject(Contract):
    binding: SubjectBinding
    subject: Subject


class PurgeNamespace(Operation):
    expected_revision: int | None = Field(default=None, ge=1)


class NamespacePurgeResult(Contract):
    namespace_id: str
    commit_seq: int
    deleted_rows: int
    released_hashes: tuple[str, ...] = ()
