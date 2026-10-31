"""Read one Namespace's Authority and portable scheduler facts from a SQLite snapshot."""

import sqlite3
from contextlib import closing
from pathlib import Path

from nous_sdk.contracts import DomainError
from nous_sdk.contracts.namespace_backups import NamespaceState
from pydantic import JsonValue, TypeAdapter

type RawRow = dict[str, str | int | float | None]
_ROW = TypeAdapter(RawRow)
_OBJECT = TypeAdapter(dict[str, JsonValue])


def quoted(name: str) -> str:
    return '"' + name.replace('"', '""') + '"'


def capture(database: Path, namespace_id: str) -> NamespaceState:
    with closing(sqlite3.connect(f"{database.as_uri()}?mode=ro", uri=True)) as connection:
        connection.row_factory = sqlite3.Row
        tables = {
            row[0]: {
                column[1] for column in connection.execute(f"PRAGMA table_info({quoted(row[0])})")
            }
            for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'")
        }
        rows: dict[str, list[RawRow]] = {}
        for table, columns in tables.items():
            if "namespace_id" in columns:
                rows[table] = [
                    _ROW.validate_python(dict(row))
                    for row in connection.execute(
                        f"SELECT * FROM {quoted(table)} WHERE namespace_id=?", (namespace_id,)
                    )
                ]
        rows["namespaces"] = [
            _ROW.validate_python(dict(row))
            for row in connection.execute("SELECT * FROM namespaces WHERE id=?", (namespace_id,))
        ]
        if len(rows["namespaces"]) != 1:
            raise DomainError("namespace_not_found", "Namespace does not exist in the snapshot")

        def child(table: str, column: str, parent: str, parent_column: str = "id") -> None:
            if table not in tables:
                return
            values = {row[parent_column] for row in rows.get(parent, [])}
            selected = [
                _ROW.validate_python(dict(row))
                for row in connection.execute(f"SELECT * FROM {quoted(table)}")
                if row[column] in values
            ]
            if "namespace_id" in tables[table]:
                selected = list(
                    {
                        str(row.get("key", row.get("id"))): row
                        for row in (*rows.get(table, []), *selected)
                    }.values()
                )
            rows[table] = selected

        child("context_feedback", "context_id", "cognitive_contexts")
        child("import_inputs", "catalog_id", "import_catalogs")
        child("import_units", "catalog_id", "import_catalogs")
        child("import_outputs", "unit_id", "import_units")
        child("import_manifest_heads", "success_id", "import_file_successes")
        child("vector_projection_claims", "request_id", "vector_projection_requests")
        child("vector_projection_subscriptions", "request_id", "vector_projection_requests")
        child("nous_model_invocations", "call_id", "nous_model_calls")
        archived_call_ids = {row["id"] for row in rows.get("nous_imported_model_calls", [])}
        if archived_call_ids:
            retained = [
                _ROW.validate_python(dict(row))
                for row in connection.execute("SELECT * FROM nous_model_invocations")
                if row["call_id"] in archived_call_ids
            ]
            rows["nous_model_invocations"] = list(
                {
                    str(row["key"]): row
                    for row in (*rows.get("nous_model_invocations", []), *retained)
                }.values()
            )
        child("nous_model_invocation_retirements", "key", "nous_model_invocations", "key")
        rows.pop("namespace_backup_uploads", None)
        subject_ids = {row["id"] for row in rows.get("subjects", [])}
        scopes = {f"namespace:{namespace_id}", *(f"subject:{subject}" for subject in subject_ids)}
        for table in ("nous_configuration", "nous_configuration_history"):
            rows[table] = [
                _ROW.validate_python(dict(row))
                for row in connection.execute(f"SELECT * FROM {quoted(table)}")
                if row["scope"] in scopes
            ]
        for table in ("import_requests",):
            rows[table] = []
            for row in connection.execute(f"SELECT * FROM {quoted(table)}"):
                value = _OBJECT.validate_json(str(row["record"]))
                context_raw = value.get("context", {})
                context = context_raw if isinstance(context_raw, dict) else {}
                if (
                    context.get("namespace_id") == namespace_id
                    or value.get("namespace_id") == namespace_id
                ):
                    rows[table].append(_ROW.validate_python(dict(row)))
        references = {
            f"{row['kind']}:{row['id']}@{row['revision']}"
            for row in rows.get("cognitive_revisions", [])
        }
        references.update(f"material:{row['id']}" for row in rows.get("materials", []))
        references.update(f"source-region:{row['id']}" for row in rows.get("source_regions", []))
        references.update(
            f"representation:{row['id']}@{row['revision']}"
            for row in rows.get("derived_representations", [])
        )
        references.update(f"derived-region:{row['id']}" for row in rows.get("derived_regions", []))
        rows["cognitive_vectors"] = [
            _ROW.validate_python(dict(row))
            for row in connection.execute("SELECT * FROM cognitive_vectors")
            if row["reference"] in references
        ]

        workflow_ids = {row["workflow_id"] for row in rows.get("nous_job_links", [])}
        if "workflow_status" in tables:
            all_statuses = [
                _ROW.validate_python(dict(row))
                for row in connection.execute("SELECT * FROM workflow_status")
            ]
            while True:
                owned = {
                    row["workflow_uuid"]
                    for row in all_statuses
                    if row.get("parent_workflow_id") in workflow_ids
                }
                if owned.issubset(workflow_ids):
                    break
                workflow_ids.update(owned)
            rows["workflow_status"] = [
                row for row in all_statuses if row["workflow_uuid"] in workflow_ids
            ]
            for status in rows["workflow_status"]:
                status["request"] = None
            for table, column in (
                ("workflow_input", "workflow_uuid"),
                ("workflow_output", "workflow_uuid"),
                ("operation_outputs", "workflow_uuid"),
                ("workflow_events", "workflow_uuid"),
                ("workflow_events_history", "workflow_uuid"),
                ("notifications", "destination_uuid"),
                ("streams", "workflow_uuid"),
            ):
                if table in tables:
                    rows[table] = [
                        _ROW.validate_python(dict(row))
                        for row in connection.execute(f"SELECT * FROM {quoted(table)}")
                        if row[column] in workflow_ids
                    ]
            for table in rows:
                if table in {
                    "workflow_status",
                    "workflow_input",
                    "workflow_output",
                    "operation_outputs",
                    "workflow_events",
                    "workflow_events_history",
                    "notifications",
                    "streams",
                }:
                    for row in rows[table]:
                        if row.get("serialization") not in {None, "portable_json"}:
                            raise DomainError(
                                "namespace_backup_serialization",
                                "Namespace workflows require portable JSON serialization",
                            )
            schedule_names = {
                row["schedule_name"] for row in rows["workflow_status"] if row.get("schedule_name")
            }
            rows["workflow_schedules"] = []
            for row in connection.execute("SELECT * FROM workflow_schedules"):
                value = _OBJECT.validate_json(str(row["context"]))
                arguments_raw = value.get("args", [])
                arguments = arguments_raw if isinstance(arguments_raw, list) else []
                if (
                    row["schedule_name"] in schedule_names
                    or value.get("namespace_id") == namespace_id
                    or value.get("request_id")
                    in {row["id"] for row in rows.get("context_preparations", [])}
                    or value.get("schedule_id")
                    in {row["id"] for row in rows.get("maintenance_schedules", [])}
                    or any(
                        isinstance(item, dict) and item.get("namespace_id") == namespace_id
                        for item in arguments
                    )
                ):
                    rows["workflow_schedules"].append(_ROW.validate_python(dict(row)))
        commit = connection.execute(
            "SELECT commit_seq FROM nous_commit_clock WHERE singleton=1"
        ).fetchone()[0]
        return NamespaceState.model_validate(
            {"namespace_id": namespace_id, "authority_commit_seq": commit, "tables": rows}
        )
