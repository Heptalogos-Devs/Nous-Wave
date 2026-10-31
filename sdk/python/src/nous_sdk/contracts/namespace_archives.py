"""Scoped semantic archives compose native Subject bundles and identity metadata."""

from typing import Literal

from pydantic import JsonValue

from .common import Contract, Operation
from .identity import Namespace, Subject, SubjectBinding
from .management import SubjectBundle


class ArchivedSubject(Contract):
    subject: Subject
    bundle: SubjectBundle


class ArchivedConfiguration(Contract):
    scope: Literal["namespace", "subject"]
    subject_id: str | None = None
    key: str
    value: JsonValue


class NamespaceArchive(Contract):
    format: Literal["nous.namespace"] = "nous.namespace"
    version: Literal[1] = 1
    content_scope: Literal["semantic_subjects"] = "semantic_subjects"
    namespace: Namespace
    authority_commit_seq: int
    subjects: tuple[ArchivedSubject, ...]
    bindings: tuple[SubjectBinding, ...] = ()
    configuration: tuple[ArchivedConfiguration, ...] = ()


class RestoreNamespace(Operation):
    archive: NamespaceArchive
    name: str | None = None


class NamespaceRestoreResult(Contract):
    namespace: Namespace
    subjects: dict[str, str]
    installations: dict[str, dict[str, JsonValue]]


class MigrateSubject(Operation):
    target_namespace_id: str
    name: str | None = None


class SubjectMigrationResult(Contract):
    source_subject_id: str
    subject: Subject
    installation: dict[str, JsonValue]
