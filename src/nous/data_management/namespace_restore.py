"""Atomic restoration of a scoped Namespace snapshot; copies retain paid custody."""

import hashlib
import json
from typing import Literal
from uuid import NAMESPACE_URL, uuid5

import sqlalchemy as sa
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.identity import Namespace, Subject, SubjectBinding
from nous_sdk.contracts.model_calls import ArchivedCall, CallOrigin, InvocationBinding
from nous_sdk.contracts.namespace_backups import (
    NamespaceBackupRestored,
    NamespaceState,
    RestoreNamespaceBackup,
)
from nous_sdk.contracts.references import SemanticTarget
from pydantic import JsonValue, TypeAdapter

from nous.identity.service import IdentityService
from nous.platform.content import ContentLease, ContentStore
from nous.platform.models.history import imported_calls
from nous.platform.models.invocation_transfer import restore_in as restore_invocations_in
from nous.platform.models.source_fence import source_identities
from nous.platform.persistence import Authority, metadata
from nous.platform.persistence.authority import canonical_json, operation_key
from nous.platform.persistence.schema import clock
from nous.references.allocation import allocate_target_in

from .namespace_state import quoted

_OBJECT = TypeAdapter(dict[str, JsonValue])
_TARGET = TypeAdapter(SemanticTarget)
_SCHEDULER = {
    "workflow_status",
    "workflow_input",
    "workflow_output",
    "operation_outputs",
    "workflow_events",
    "workflow_events_history",
    "notifications",
    "streams",
    "workflow_schedules",
}
_CHILDREN = {
    "context_feedback",
    "import_inputs",
    "import_units",
    "import_outputs",
    "import_manifest_heads",
    "import_requests",
    "cognitive_vectors",
    "vector_projection_claims",
    "vector_projection_subscriptions",
    "nous_model_invocations",
    "nous_model_invocation_retirements",
    "nous_configuration",
    "nous_configuration_history",
}
_JSON = {
    "record",
    "context",
    "value",
    "previous_value",
    "target",
    "request",
    "result",
    "usage",
    "vector",
    "source_references",
    "payload_json",
    "result_json",
    "jobs_json",
    "attributes",
    "inputs",
    "output",
    "error",
    "message",
}
_IDENTITIES = {
    "id",
    "assessment_id",
    "motivation_assessment_id",
    "namespace_id",
    "subject_id",
    "consumer_subject_id",
    "task_id",
    "job_id",
    "ranking",
    "material_id",
    "region_id",
    "representation_id",
    "observation_id",
    "context_id",
    "track_id",
    "session_id",
    "preparation_id",
    "admission_id",
    "request_id",
    "catalog_id",
    "unit_id",
    "output_id",
    "input_source_id",
    "success_id",
    "parent_catalog_id",
    "contact_observation_id",
    "previous_observation_id",
    "retry_parent_job_id",
    "producer_call_id",
    "model_call_id",
    "call_id",
    "workflow_id",
    "workflow_uuid",
    "destination_uuid",
    "message_uuid",
    "owned_parent",
    "child_workflow_id",
    "parent_workflow_id",
    "forked_from",
    "installation_id",
    "bundle_id",
    "content_key",
    "key",
    "operation_key",
    "memory_subject_id",
    "schedule_id",
    "schedule_name",
    "scope",
    "owner_ref",
    "address",
    "object",
    "exact",
    "reference",
    "references",
    "source_ids",
    "source_roots",
    "support_roots",
    "material_ids",
    "input_source_ids",
    "required_source_ids",
    "required_observation_ids",
    "credential_source_ids",
    "allowed_source_ids",
    "accepted_observation_ids",
    "source_material_ids",
    "model_call_ids",
    "refs",
    "unit_outputs",
}
_LITERAL = {
    "origin",
    "identifiers",
    "metadata",
    "structured",
    "external_ids",
    "provenance",
    "source_origin",
    "request_hash",
    "text",
    "context_text",
    "rendered",
    "filename",
    "source",
    "external_event_id",
    "external_id",
}


def document(value: JsonValue) -> JsonValue:
    return json.loads(value) if isinstance(value, str) else value


def validate(state: NamespaceState) -> None:
    allowed = (
        {
            name
            for name, table in metadata.tables.items()
            if "namespace_id" in table.c and name != "namespace_backup_uploads"
        }
        | _CHILDREN
        | _SCHEDULER
        | {"namespaces"}
    )
    if set(state.tables) - allowed:
        raise DomainError(
            "namespace_backup_integrity",
            "Backup includes data outside Namespace ownership",
            tables=sorted(set(state.tables) - allowed),
        )
    for name, rows in state.tables.items():
        for row in rows:
            if (
                "namespace_id" in row
                and row["namespace_id"] != state.namespace_id
                and not (
                    name in {"nous_model_invocations", "nous_model_invocation_retirements"}
                    and row["namespace_id"] is None
                )
            ):
                raise DomainError(
                    "namespace_backup_scope_forbidden",
                    "Backup row belongs to another Namespace",
                    table=name,
                )
    if (
        len(state.tables.get("namespaces", ())) != 1
        or state.tables["namespaces"][0]["id"] != state.namespace_id
    ):
        raise DomainError(
            "namespace_backup_integrity", "Backup has no single consistent Namespace identity"
        )
    owners = {str(row["id"]) for row in state.tables.get("subjects", ())}

    def trusted_contexts(value: JsonValue) -> None:
        if isinstance(value, dict):
            if {"caller_id", "namespace_id", "credential_source_ids"}.issubset(value):
                if (
                    value["namespace_id"] != state.namespace_id
                    or value.get("subject_id") is not None
                    and value["subject_id"] not in owners
                ):
                    raise DomainError(
                        "namespace_backup_scope_forbidden",
                        "Saved work contains a context outside the archived Namespace",
                    )
            for field, item in value.items():
                if field not in _LITERAL:
                    trusted_contexts(item)
        elif isinstance(value, list):
            for item in value:
                trusted_contexts(item)

    for rows in state.tables.values():
        for row in rows:
            if row.get("subject_id") is not None and str(row["subject_id"]) not in owners:
                raise DomainError(
                    "namespace_backup_scope_forbidden",
                    "Backup row belongs to an unretained Subject",
                )
            for field in ("context", "inputs"):
                if row.get(field) is not None:
                    trusted_contexts(document(row[field]))
            record = row.get("record")
            if record is not None:
                decoded = document(record)
                if isinstance(decoded, dict) and "context" in decoded:
                    trusted_contexts(decoded["context"])
    scopes = {"namespace:" + state.namespace_id, *("subject:" + subject for subject in owners)}
    for name in ("nous_configuration", "nous_configuration_history"):
        if any(row["scope"] not in scopes for row in state.tables.get(name, ())):
            raise DomainError(
                "namespace_backup_scope_forbidden", "Backup includes foreign configuration"
            )
    for child, field, parent, parent_field in (
        ("context_feedback", "context_id", "cognitive_contexts", "id"),
        ("import_inputs", "catalog_id", "import_catalogs", "id"),
        ("import_units", "catalog_id", "import_catalogs", "id"),
        ("import_outputs", "unit_id", "import_units", "id"),
        ("import_manifest_heads", "success_id", "import_file_successes", "id"),
        ("vector_projection_claims", "request_id", "vector_projection_requests", "id"),
        ("vector_projection_subscriptions", "request_id", "vector_projection_requests", "id"),
    ):
        identities = {str(row[parent_field]) for row in state.tables.get(parent, ())}
        if any(str(row[field]) not in identities for row in state.tables.get(child, ())):
            raise DomainError(
                "namespace_backup_integrity", "Backup has an unresolved owned row", table=child
            )
    job_requests = {
        "nous.imports.process": "import_requests",
        "nous.imports.text": "import_requests",
        "nous.imports.form_observation": "import_requests",
        "nous.vectors.project": "vector_projection_requests",
        "nous.context.prepare": "context_preparations",
        "nous.context.maintain": "context_preparations",
        "nous.maintenance.generate": "maintenance_runs",
    }
    cached_inputs = {
        str(row["workflow_uuid"]): row.get("inputs")
        for row in state.tables.get("workflow_input", ())
    }
    for row in state.tables.get("workflow_status", ()):
        if row.get("serialization") not in {None, "portable_json"}:
            raise DomainError(
                "namespace_backup_serialization", "Restored workflows require portable JSON"
            )
        inputs = document(row.get("inputs") or cached_inputs.get(str(row["workflow_uuid"])))
        values = inputs if isinstance(inputs, dict) else {}
        arguments = values.get("positionalArgs", [])
        if row.get("name") in job_requests:
            expected = {
                str(item["id"]) for item in state.tables.get(job_requests[str(row["name"])], ())
            }
            if (
                not isinstance(arguments, list)
                or not arguments
                or not isinstance(arguments[0], str)
                or arguments[0] not in expected
            ):
                raise DomainError(
                    "namespace_backup_scope_forbidden",
                    "Saved workflow names a request outside the Namespace snapshot",
                    workflow_id=row["workflow_uuid"],
                )


async def restore(
    authority: Authority,
    identity_owner: IdentityService,
    content: ContentStore,
    ctx: CallContext,
    command: RestoreNamespaceBackup,
    state: NamespaceState,
    leases: dict[str, ContentLease],
) -> NamespaceBackupRestored:
    validate(state)
    if command.preserve_ids and ctx.namespace_id != state.namespace_id:
        raise DomainError(
            "namespace_backup_identity",
            "Identity-preserving restore requires the original Namespace identity",
        )
    if ctx.namespace_id == "instance":
        raise DomainError("namespace_backup_scope_forbidden", "Select a new receiving Namespace")
    async with authority.write(
        ctx,
        command.operation_id,
        operation_kind="namespace.backup.restore",
        scope=ctx.namespace_id,
        payload=command,
    ) as tx:
        if tx.replayed:
            return NamespaceBackupRestored.model_validate(tx.replay_result)
        original = state.tables
        mapping: dict[str, str] = {state.namespace_id: ctx.namespace_id}

        def identity(value: str) -> str:
            return (
                value
                if command.preserve_ids
                else str(
                    uuid5(
                        NAMESPACE_URL,
                        f"nous:namespace-copy:{ctx.namespace_id}:{command.backup_id}:{value}",
                    )
                )
            )

        for name, rows in original.items():
            for row in rows:
                for field in (
                    "id",
                    "workflow_uuid",
                    "message_uuid",
                    "schedule_id",
                    "schedule_name",
                ):
                    value = row.get(field)
                    if isinstance(value, str) and (field != "id" or name != "namespaces"):
                        mapping.setdefault(value, identity(value))
                if name in {
                    "vector_projection_contents",
                    "vector_projection_claims",
                    "import_manifest_heads",
                } and isinstance(key := row.get("key"), str):
                    mapping.setdefault(key, identity(key))
        old_sequences = sorted(
            {
                TypeAdapter(int).validate_python(row["commit_seq"])
                for rows in original.values()
                for row in rows
                if isinstance(row.get("commit_seq"), int)
            }
        )
        sequences = {old: tx.commit_seq + index + 1 for index, old in enumerate(old_sequences)}
        final_seq = tx.commit_seq + len(sequences)

        def token(value: str) -> str:
            if value in mapping:
                return mapping[value]
            if ":" in value:
                kind, rest = value.split(":", 1)
                identifier, at, revision = rest.partition("@")
                if identifier in mapping:
                    return kind + ":" + mapping[identifier] + ("@" + revision if at else "")
                nested = token(rest)
                if nested != rest:
                    return kind + ":" + nested
            return value

        def remap(value: JsonValue, field: str = "", *, arguments: bool = False) -> JsonValue:
            if (
                field == "source"
                and isinstance(value, dict)
                and value.get("kind")
                in {"material", "source_region", "representation", "derived_region"}
                and isinstance(value.get("id"), str)
            ):
                return {key: remap(item, key) for key, item in value.items()}
            if field == "provenance" and isinstance(value, dict):
                return {
                    key: remap(item, key)
                    if key in {"motivation_assessment_id", "motivation_assessment_commit_seq"}
                    else item
                    for key, item in value.items()
                }
            if field in _LITERAL:
                return value
            if isinstance(value, dict):
                if field == "relevance":
                    return {token(key): item for key, item in value.items()}
                if field == "unit_outputs":
                    return {token(key): remap(item, "output_id") for key, item in value.items()}
                if field == "source_regions":
                    return {token(key): remap(item, "reference") for key, item in value.items()}
                if (
                    not command.preserve_ids
                    and "caller_id" in value
                    and "namespace_id" in value
                    and "credential_source_ids" in value
                ):
                    value = {
                        **value,
                        "caller_id": ctx.caller_id,
                        "credential_source_ids": list(ctx.credential_source_ids)
                        if ctx.credential_source_ids is not None
                        else None,
                    }
                return {
                    key: remap(
                        item,
                        key,
                        arguments=arguments or key in {"args", "positionalArgs", "namedArgs"},
                    )
                    for key, item in value.items()
                }
            if isinstance(value, list):
                return [remap(item, field, arguments=arguments) for item in value]
            if isinstance(value, int) and (
                field == "commit_seq"
                or field.endswith("_commit_seq")
                or field in {"as_of_commit", "authority_commit_seq", "admitted_commit"}
            ):
                return sequences.get(value, value)
            if isinstance(value, str) and (
                field in _IDENTITIES or field.endswith("_ids") or arguments
            ):
                return token(value)
            return value

        # Namespace identities are accepted by the lifecycle owner in this same transaction.
        ns = Namespace.model_validate(remap(dict(original["namespaces"][0])))
        restored_subjects = tuple(
            Subject.model_validate(
                remap({**dict(row), "capabilities": document(row["capabilities"])})
            )
            for row in original.get("subjects", ())
        )
        restored_bindings = tuple(
            SubjectBinding.model_validate(remap(dict(row)))
            for row in original.get("subject_bindings", ())
        )
        await identity_owner.restore_directory_in(tx, ns, restored_subjects, restored_bindings)

        # Reallocate readable names only for a new identity. Exact installation restore retains them.
        if not command.preserve_ids:
            for row in original.get("reference_addresses", ()):
                target = _TARGET.validate_python(remap(document(row["target"])))
                mapping[str(row["address"])] = await allocate_target_in(tx, ctx, target)
        paused: list[str] = []
        skip = (
            {
                "namespaces",
                "subjects",
                "subject_bindings",
                "nous_content_references",
                "reference_addresses",
            }
            if not command.preserve_ids
            else {"namespaces", "subjects", "subject_bindings", "nous_content_references"}
        )
        skip |= {
            "nous_model_calls",
            "nous_model_invocations",
            "nous_model_invocation_retirements",
        }
        for raw in original.get("nous_model_calls", ()):
            request = _OBJECT.validate_python(document(raw["request"]))
            archived = ArchivedCall(
                id=mapping[str(raw["id"])],
                origin=CallOrigin(
                    namespace_id=state.namespace_id,
                    subject_id=str(raw["subject_id"]) if raw["subject_id"] is not None else None,
                    call_id=str(raw["id"]),
                ),
                purpose=str(raw["purpose"]),
                request_hash=str(raw["request_hash"]),
                day=str(raw["day"]),
                status=TypeAdapter(
                    Literal["reserved", "succeeded", "failed", "purged"]
                ).validate_python(raw["status"]),
                request=request,
                result=document(raw["result"]),
                usage=TypeAdapter(dict[str, JsonValue] | None).validate_python(
                    document(raw["usage"])
                ),
                error_code=str(raw["error_code"]) if raw["error_code"] else None,
                source_ids=tuple(token(item) for item in sorted(source_identities(request))),
            )
            if raw["subject_id"] is None:
                raise DomainError(
                    "namespace_backup_model_scope",
                    "Namespace model custody requires a retained Subject",
                )
            await tx.connection.execute(
                imported_calls.insert().values(
                    id=archived.id,
                    namespace_id=ctx.namespace_id,
                    subject_id=mapping[str(raw["subject_id"])],
                    installation_id=command.operation_id,
                    archive_id=str(raw["id"]),
                    record=archived.model_dump(mode="json"),
                )
            )
        tables = sorted(
            original,
            key=lambda name: (
                0 if name == "workflow_status" else 1 if name == "nous_operations" else 2,
                name,
            ),
        )
        for name in tables:
            if name in skip:
                continue
            columns = {
                str(item[1])
                for item in (
                    await tx.connection.exec_driver_sql(f"PRAGMA table_info({quoted(name)})")
                ).all()
            }
            if not columns:
                raise DomainError(
                    "namespace_backup_migration_required",
                    "Backup requires an unavailable table",
                    table=name,
                )
            for raw in original[name]:
                if set(raw) - columns:
                    raise DomainError(
                        "namespace_backup_migration_required",
                        "Backup table requires unavailable fields",
                        table=name,
                    )
                values: dict[str, JsonValue] = {}
                for field, value in raw.items():
                    if field in _JSON and isinstance(value, str):
                        try:
                            decoded = TypeAdapter(JsonValue).validate_json(value)
                            changed = remap(decoded, field)
                            if (
                                name == "nous_imported_model_calls"
                                and field == "record"
                                and isinstance(decoded, dict)
                                and isinstance(changed, dict)
                            ):
                                changed = {
                                    **changed,
                                    **{
                                        key: decoded[key]
                                        for key in (
                                            "origin",
                                            "request",
                                            "request_hash",
                                            "result",
                                            "usage",
                                        )
                                        if key in decoded
                                    },
                                }
                            values[field] = canonical_json(changed)
                        except ValueError, TypeError:
                            if field in {"output", "error", "message"}:
                                values[field] = value
                            else:
                                raise DomainError(
                                    "namespace_backup_integrity",
                                    "Backup contains invalid stored JSON",
                                    table=name,
                                    field=field,
                                ) from None
                    else:
                        values[field] = remap(value, field)
                if name == "material_description_deliveries":
                    delivery = _OBJECT.validate_python(document(values["record"]))
                    status = delivery.get("status")
                    delivery.update({"lease_token": None, "lease_until": None})
                    if status == "running" or (
                        not command.preserve_ids and status in {"pending", "failed"}
                    ):
                        delivery["status"] = "pending"
                    if not command.preserve_ids and status in {"pending", "running", "failed"}:
                        delivery["attempt_count"] = 0
                    values["record"] = canonical_json(delivery)
                if name == "nous_operations":
                    operation_ctx = ctx.model_copy(
                        update={
                            "caller_id": str(values["caller_id"]),
                            "subject_id": str(values["subject_id"])
                            if values.get("subject_id") is not None
                            else None,
                        }
                    )
                    old_key = str(raw["operation_key"])
                    values["operation_key"] = operation_key(
                        operation_ctx,
                        str(values["operation_kind"]),
                        str(values["scope"]),
                        str(values["operation_id"]),
                    )
                    mapping[old_key] = str(values["operation_key"])
                    if values.get("payload_json") is not None:
                        values["payload_hash"] = hashlib.sha256(
                            canonical_json(
                                {
                                    "kind": values["operation_kind"],
                                    "payload": json.loads(str(values["payload_json"])),
                                }
                            ).encode()
                        ).hexdigest()
                if name == "nous_job_links":
                    values["operation_key"] = mapping.get(
                        str(raw["operation_key"]), str(raw["operation_key"])
                    )
                    if not command.preserve_ids:
                        values["caller_id"] = ctx.caller_id
                if name == "workflow_status":
                    values["request"] = None
                    values["executor_id"] = None
                    values["owner_xid"] = None
                    if values.get("status") in {"PENDING", "ENQUEUED", "DELAYED"}:
                        paused.append(str(values["workflow_uuid"]))
                        attributes = _OBJECT.validate_python(
                            document(values.get("attributes")) or {}
                        )
                        attributes["namespace_restore"] = {
                            "backup_id": command.backup_id,
                            "source_namespace_id": state.namespace_id,
                            "original_status": values["status"],
                            "copy": not command.preserve_ids,
                            "requires_review": not command.preserve_ids,
                        }
                        values["attributes"] = canonical_json(attributes)
                        values["status"] = "CANCELLED"
                    if not command.preserve_ids:
                        values["authenticated_user"] = ctx.caller_id
                if name == "workflow_schedules":
                    values["status"] = "PAUSED"
                registered = metadata.tables.get(name)
                if not command.preserve_ids and registered is not None:
                    primary = tuple(registered.primary_key.columns)
                    if (
                        len(primary) == 1
                        and primary[0].name == "id"
                        and isinstance(primary[0].type, sa.Integer)
                    ):
                        values.pop("id", None)
                names = tuple(values)
                await tx.connection.exec_driver_sql(
                    f"INSERT INTO {quoted(name)} ({','.join(quoted(field) for field in names)}) VALUES ({','.join('?' for _ in names)})",
                    tuple(values[field] for field in names),
                )
        frozen = 0
        unresolved: tuple[str, ...] = ()
        bindings = tuple(
            InvocationBinding.model_validate(
                {
                    **{key: value for key, value in row.items() if key != "commit_seq"},
                    "retired": name == "nous_model_invocation_retirements",
                }
            )
            for name in ("nous_model_invocations", "nous_model_invocation_retirements")
            for row in original.get(name, ())
        )
        report = await restore_invocations_in(
            tx,
            ctx,
            bindings,
            subject_mapping=mapping,
            caller_mapping={
                binding.caller_id: binding.caller_id if command.preserve_ids else ctx.caller_id
                for binding in bindings
                if binding.caller_id is not None
            },
            call_mapping=mapping,
            preserve_opaque=command.preserve_ids,
            source_namespace_id=state.namespace_id,
        )
        from nous.cognition.entity_profile_migration import migrate as migrate_entity_profiles
        from nous.cognition.schema import revisions as cognitive_revisions

        await tx.connection.run_sync(
            lambda connection: migrate_entity_profiles(connection, cognitive_revisions)
        )
        frozen = report.frozen
        unresolved = tuple(binding.key for binding in report.opaque)
        if unresolved:
            for workflow_id in paused:
                attributes_row = await tx.connection.exec_driver_sql(
                    "SELECT attributes FROM workflow_status WHERE workflow_uuid=?", (workflow_id,)
                )
                attributes = _OBJECT.validate_json(str(attributes_row.scalar_one()))
                marker = _OBJECT.validate_python(attributes["namespace_restore"])
                attributes["namespace_restore"] = {
                    **marker,
                    "requires_review": True,
                    "unresolved_invocations": list(unresolved),
                }
                await tx.connection.exec_driver_sql(
                    "UPDATE workflow_status SET attributes=? WHERE workflow_uuid=?",
                    (canonical_json(attributes), workflow_id),
                )
        for raw in original.get("nous_content_references", ()):
            await content.attach_in(
                tx, leases[str(raw["content_hash"])], token(str(raw["owner_ref"]))
            )
        await tx.connection.execute(
            clock.update().where(clock.c.singleton == 1).values(commit_seq=final_seq)
        )
        return tx.result(
            NamespaceBackupRestored(
                namespace=ns,
                mapping=mapping,
                paused_jobs=tuple(paused),
                authority_commit_seq=final_seq,
                frozen_invocations=frozen,
                unresolved_invocations=unresolved,
            )
        )
