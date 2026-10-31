"""Image task presentation selects existing source formation and vector jobs."""

from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING

from nous_sdk.amemorix.contracts import ImageInput
from nous_sdk.contracts import CallContext, MediaReference
from pydantic import JsonValue

from .helpers import json_object

if TYPE_CHECKING:
    from .images import ImageAsset
    from .service import AMemorixService


async def job_groups(
    service: AMemorixService, ctx: CallContext, assets: tuple[ImageAsset, ...]
) -> dict[str, tuple[str, ...]]:
    observations = tuple(
        event["id"] for asset in assets for event in asset.events if event["kind"] == "contact"
    )
    source_refs = tuple(
        MediaReference(kind="material", id=event["material_id"], observation_id=event["id"])
        for asset in assets
        for event in asset.events
    )
    roots = set(observations)
    cognition = tuple(
        record.reference
        for record in await service.records(ctx, ("memory",))
        if roots.intersection(record.source_roots)
    )
    return {
        "nous.imports.form_observation": await service.app.imports.jobs_for_observations(
            ctx, observations
        ),
        "nous.vectors.project": await service.app.vector_projection.source_jobs(
            ctx, (*source_refs, *cognition)
        ),
    }


async def image_jobs(
    service: AMemorixService, ctx: CallContext, command: ImageInput, assets: tuple[ImageAsset, ...]
) -> dict[str, JsonValue]:
    kinds = await job_groups(service, ctx, assets)
    if command.action in {"process_jobs", "retry_failed_jobs"}:
        results: dict[str, JsonValue] = {}
        for kind, identifiers in kinds.items():
            if command.action == "process_jobs":
                results[kind] = await service.app.execution.process_jobs(
                    ctx, kind=kind, workflow_ids=identifiers, limit=command.limit
                )
            else:
                results[kind] = await service.app.execution.retry_jobs(
                    ctx, kind=kind, workflow_ids=identifiers, limit=command.limit
                )
        if command.action == "retry_failed_jobs":
            results[
                "description_deliveries"
            ] = await service.app.description_deliveries.retry_failed(ctx, command.operation_id)
        return json_object(
            {
                "success": True,
                "counts": results,
                "count": sum(value for value in results.values() if isinstance(value, int)),
            }
        )
    items: list[JsonValue] = []
    states = {
        "pending": "ENQUEUED",
        "running": "PENDING",
        "done": "SUCCESS",
        "failed": "ERROR",
        "cancelled": "CANCELLED",
    }
    for kind, identifiers in kinds.items():
        cursor = None
        while True:
            page = await service.app.execution.query_jobs(
                ctx,
                kind=kind,
                workflow_ids=identifiers,
                state=None
                if command.status in {"pending", "failed"}
                else states.get(command.status or "", command.status),
                limit=1000,
                cursor=cursor,
            )
            output_states = {
                "PENDING": "running",
                "ENQUEUED": "pending",
                "DELAYED": "pending",
                "SUCCESS": "done",
                "ERROR": "failed",
                "MAX_RECOVERY_ATTEMPTS_EXCEEDED": "failed",
                "CANCELLED": "cancelled",
            }
            items.extend(
                {
                    "job_id": job.workflow_id,
                    "id": job.workflow_id,
                    "kind": "embedding" if kind == "nous.vectors.project" else "formation",
                    "job_type": kind,
                    "status": output_states[job.status],
                    "native_status": job.status,
                    "attempt_count": job.recovery_attempts,
                    "created_at": job.created_at / 1000 if job.created_at is not None else None,
                    "updated_at": job.updated_at / 1000 if job.updated_at is not None else None,
                    "last_error": str(job.error) if job.error else "",
                }
                for job in page.items
                if command.status not in {"pending", "failed"}
                or output_states[job.status] == command.status
            )
            if page.next_cursor is None:
                break
            cursor = page.next_cursor
    items.extend(
        {
            **item,
            "created_at": datetime.fromisoformat(str(item["created_at"])).timestamp(),
            "updated_at": datetime.fromisoformat(
                str(item.get("updated_at", item["created_at"]))
            ).timestamp(),
        }
        for item in await service.app.description_deliveries.items(ctx)
        if command.status is None or item["status"] == command.status
    )
    items.sort(
        key=lambda item: str(item.get("created_at")) if isinstance(item, dict) else "", reverse=True
    )
    return {
        "success": True,
        "items": items[command.offset : command.offset + command.limit],
        "total": len(items),
    }
