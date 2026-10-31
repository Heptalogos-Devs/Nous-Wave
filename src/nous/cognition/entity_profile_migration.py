"""Classify retained automatic profile revisions using their actual revision cause."""

from __future__ import annotations

from typing import TYPE_CHECKING

import sqlalchemy as sa
from pydantic import JsonValue, TypeAdapter
from sqlalchemy import event

if TYPE_CHECKING:
    from sqlalchemy import Connection

_OBJECT = TypeAdapter(dict[str, JsonValue])


def migrate(connection: Connection, revisions: sa.Table) -> None:
    legacy = connection.scalar(
        sa.select(revisions.c.id)
        .where(
            revisions.c.kind == "entity",
            revisions.c.record["cause"]["action"]
            .as_string()
            .in_(("person_profile.refresh", "person_profile.withdraw")),
            sa.func.json_type(revisions.c.record, "$.body.generated_profile").is_(None),
        )
        .limit(1)
    )
    if legacy is None:
        return
    previous_raw: dict[str, JsonValue] | None = None
    entity: tuple[str, str, str] | None = None
    base_description: JsonValue = ""
    base_aliases: JsonValue = []
    base_evidence: JsonValue = []
    generated: JsonValue = None
    inherited = False
    rows = connection.execute(
        sa.select(
            revisions.c.id,
            revisions.c.revision,
            revisions.c.namespace_id,
            revisions.c.subject_id,
            revisions.c.record,
        )
        .where(revisions.c.kind == "entity")
        .order_by(
            revisions.c.namespace_id, revisions.c.subject_id, revisions.c.id, revisions.c.revision
        )
    ).all()
    for row in rows:
        identity = (str(row.namespace_id), str(row.subject_id), str(row.id))
        raw = _OBJECT.validate_python(row.record)
        body = raw.get("body")
        if not isinstance(body, dict):
            continue
        if entity != identity:
            entity, previous_raw, inherited = identity, None, False
            base_description, base_aliases, base_evidence, generated = "", [], [], None
        cause = raw.get("cause")
        action = cause.get("action") if isinstance(cause, dict) else None
        refresh = action in {"person_profile.refresh", "person_profile.withdraw"}
        explicit = "generated_profile" in body
        changed = False
        if explicit:
            base_description, base_aliases, base_evidence = (
                body.get("description", ""),
                body.get("aliases", []),
                raw.get("evidence", []),
            )
            generated = body.get("generated_profile")
            inherited = generated is not None
        elif refresh:
            support = raw.get("evidence", [])
            generated = (
                {
                    "summary": body.get("description", ""),
                    "aliases": body.get("aliases", []),
                    "evidence": support,
                    "producer_call_id": raw.get("producer_call_id"),
                }
                if action == "person_profile.refresh" and isinstance(support, list) and support
                else None
            )
            changed, inherited = True, True
        elif (
            inherited
            and previous_raw is not None
            and body.get("description", "") == previous_raw.get("description", "")
            and body.get("aliases", []) == previous_raw.get("aliases", [])
        ):
            changed = True
        else:
            base_description, base_aliases, base_evidence = (
                body.get("description", ""),
                body.get("aliases", []),
                raw.get("evidence", []),
            )
            generated, inherited = None, False
        original = dict(body)
        if changed:
            body = {
                **body,
                "description": base_description,
                "aliases": base_aliases,
                "generated_profile": generated,
            }
            value = {**raw, "body": body, "evidence": base_evidence}
            connection.execute(
                revisions.update()
                .where(revisions.c.id == row.id, revisions.c.revision == row.revision)
                .values(record=value)
            )
        previous_raw = original


def register(metadata: sa.MetaData, revisions: sa.Table) -> None:
    def after_create(target: sa.MetaData, connection: Connection, **kwargs: object) -> None:
        migrate(connection, revisions)

    event.listen(metadata, "after_create", after_create)
