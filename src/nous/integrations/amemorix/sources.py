"""Source administration projects authoritative previews, operations and lifecycle."""

from collections import defaultdict
from typing import TYPE_CHECKING
from uuid import uuid4

from nous_sdk.contracts import CallContext, Contract, DomainError, SourceCommand, SourceResult
from nous_sdk.contracts.management import SourceSelection
from pydantic import Field, JsonValue

from .helpers import json_object

if TYPE_CHECKING:
    from .service import AMemorixService


class Selection(Contract):
    action: str
    mode: str = "source"
    source: str | None = None
    sources: tuple[str, ...] = ()
    selector: JsonValue = None
    query: str | None = None
    items: tuple[str, ...] = ()
    hash: str | None = None
    hashes: tuple[str, ...] = ()
    name: str | None = None
    names: tuple[str, ...] = ()
    content: str | None = None
    paragraph_hashes: tuple[str, ...] = ()
    entity_hashes: tuple[str, ...] = ()
    relation_hashes: tuple[str, ...] = ()
    requested_by: str | None = Field(default=None, max_length=120)
    reason: str = "A-Memorix source management"
    operation_id: str | None = None
    request_id: str | None = None
    dry_run: bool = False
    force: bool = False
    cascade: bool = False
    grace_hours: float | None = None
    limit: int = Field(default=50, ge=1, le=1000)
    before_commit_seq: int | None = None


def source_label(source: dict[str, JsonValue]) -> str | None:
    from nous.material.named_sources import source_name

    return source_name(source)


async def source_admin(
    service: AMemorixService, ctx: CallContext, tool: str, arguments: dict[str, JsonValue]
) -> dict[str, JsonValue]:
    command = Selection.model_validate(arguments)
    command = command.model_copy(
        update={"action": command.action.strip().lower(), "mode": command.mode.strip().lower()}
    )
    if command.action == "purge" and tool == "memory_delete_admin":
        policy = await service.policy(ctx)
        command = command.model_copy(
            update={
                "mode": "mixed",
                "selector": None,
                "source": None,
                "sources": (),
                "query": None,
                "items": (),
                "hash": None,
                "hashes": (),
                "name": None,
                "names": (),
                "content": None,
                "paragraph_hashes": (),
                "entity_hashes": (),
                "relation_hashes": (),
                "operation_id": None,
                "grace_hours": command.grace_hours
                if command.grace_hours is not None
                else policy.orphan_sweep_grace_hours,
            }
        )
    mode = command.mode.strip().lower() or "source"
    if mode not in {"source", "paragraph", "entity", "relation", "mixed"}:
        raise DomainError("source_selection_invalid", "Unknown deletion selection mode")
    sources = [
        source for source in await service.app.sources.list(ctx) if not source.get("purged", False)
    ]
    grouped: dict[str, list[str]] = defaultdict(list)
    for source in sources:
        label = source_label(json_object(source))
        if label is not None:
            grouped[label].append(source["id"])
    if command.action == "list" and tool == "memory_source_admin":
        records = await service.records(ctx, ("memory",))
        items = [
            {
                "source": label,
                "paragraph_count": sum(
                    not record.deleted and bool(set(record.source_roots).intersection(ids))
                    for record in records
                ),
                "observation_ids": ids,
                "updated_at": max(
                    source["recorded_at"] for source in sources if source["id"] in ids
                ),
            }
            for label, ids in sorted(grouped.items())
        ]
        return json_object(
            {
                "success": True,
                "items": items,
                "count": len(items),
                "sources": items,
            }
        )
    if command.action in {"get_operation", "list_operations"}:
        if command.action == "get_operation":
            value = await selected_operation(service, ctx, command.operation_id)
            return json_object({"success": True, "operation": value})
        rows = await service.app.sources.list_operations(
            ctx, command.limit, before_commit_seq=command.before_commit_seq
        )
        return json_object(
            {
                "success": True,
                "operations": rows,
                "items": rows,
                "count": len(rows),
                "next_before_commit_seq": SourceResult.model_validate(rows[-1]["result"]).commit_seq
                if rows
                else None,
            }
        )
    from .deletion_selectors import selection

    if command.action in {"restore", "purge"} and command.operation_id:
        raw = await selected_operation(service, ctx, command.operation_id)
        prior = SourceResult.model_validate(raw["result"])
        chosen = SourceSelection(observation_ids=prior.observation_ids, references=prior.references)
    else:
        if command.action == "restore" and tool == "memory_delete_admin" and mode != "relation":
            raise DomainError(
                "operation_required",
                "Restoring this selection requires its deletion operation identity",
            )
        chosen = await selection(service, ctx, command)
    if command.action == "purge" and chosen.records is not None:
        eligible = await service.app.cognition.records(ctx, chosen.records)
        if not eligible.items:
            return {"success": True, "purged_count": 0, "deleted_count": 0}
    preview = await service.app.sources.preview(
        ctx,
        chosen.observation_ids,
        references=chosen.references,
        source_names=chosen.source_names,
        records=chosen.records,
        retention_expectations=chosen.retention_expectations,
        action="restore"
        if command.action == "restore"
        else "purge"
        if command.action == "purge"
        else "trash",
    )
    counts = {
        "paragraphs": sum(ref.kind == "memory" for ref in preview.retiring),
        "entities": sum(ref.kind == "entity" for ref in preview.retiring),
        "relations": sum(ref.kind == "association" for ref in preview.retiring),
        "sources": len(chosen.source_names),
    }
    if command.action == "preview" or command.dry_run:
        return json_object(
            {
                "success": True,
                "mode": mode,
                "preview": preview,
                "counts": counts,
                "deleted_count": len(preview.retiring),
                "native_result": preview,
            }
        )
    operation = (
        command.request_id
        or (command.operation_id if command.action not in {"restore", "purge"} else None)
        or str(uuid4())
    )
    native = SourceCommand(
        operation_id=operation,
        observation_ids=preview.observation_ids,
        references=preview.references,
        reason=command.reason.strip() or f"{tool}.{command.action}",
        requested_by=command.requested_by or tool,
        expected_preview=preview.preview_token,
        force=command.force,
        grace_hours=command.grace_hours,
        retention_expectations=chosen.retention_expectations,
    )
    if command.action in {"delete", "batch_delete", "execute"}:
        result = await service.app.sources.trash(ctx, native)
    elif command.action == "restore":
        result = await service.app.sources.restore(ctx, native)
    elif command.action == "purge":
        result = await service.app.sources.purge(ctx, native)
    else:
        raise DomainError("action_invalid", "Unknown source management action")
    return json_object(
        {
            "success": True,
            "mode": mode,
            "operation_id": result.source_operation_id,
            "deleted_count": len(result.retired),
            "restored_count": len(result.restored),
            "purged_count": len(result.references) if command.action == "purge" else 0,
            "deleted_paragraph_count": sum(ref.kind == "memory" for ref in result.retired),
            "deleted_entity_count": sum(ref.kind == "entity" for ref in result.retired),
            "deleted_relation_count": sum(ref.kind == "association" for ref in result.retired),
            "counts": counts,
            "native_result": result,
        }
    )


async def selected_operation(service: AMemorixService, ctx: CallContext, identity: str | None):
    if not identity:
        raise DomainError("operation_required", "Select a source operation")
    if identity.isdecimal():
        return await service.app.sources.get_operation(ctx, commit_seq=int(identity))
    return await service.app.sources.get_operation(ctx, identity)
