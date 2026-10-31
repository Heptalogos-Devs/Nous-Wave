"""Public tuning actions select and publish native retrieval profiles."""

from __future__ import annotations

from typing import TYPE_CHECKING
from uuid import uuid4

import tomlkit
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.retrieval_policy import RetrievalPolicy
from nous_sdk.contracts.tuning import (
    ApplyRetrievalProfile,
    CreateTuning,
    RollbackRetrievalProfile,
    TuningTask,
)
from nous_sdk.types import JsonObject
from pydantic import JsonValue, TypeAdapter

from .helpers import json_object

if TYPE_CHECKING:
    from .service import AMemorixService


def task_value(value: TuningTask) -> JsonObject:
    states = {
        "ENQUEUED": "queued",
        "DELAYED": "queued",
        "PENDING": "running",
        "SUCCESS": "completed",
        "ERROR": "failed",
        "CANCELLED": "cancelled",
        "MAX_RECOVERY_ATTEMPTS_EXCEEDED": "failed",
    }
    return json_object(
        {
            **value.model_dump(mode="json"),
            "task_id": value.id,
            "status": states[value.status],
            "progress": value.completed_trials / value.total_trials,
            "rounds_total": value.total_trials,
            "rounds_done": value.completed_trials,
            "objective": value.request.objective if value.request else None,
        }
    )


async def profile_value(
    service: AMemorixService, ctx: CallContext, raw: JsonValue
) -> RetrievalPolicy:
    supplied = TypeAdapter(JsonObject).validate_python(raw)
    if "retrieval" not in supplied:
        return RetrievalPolicy.model_validate(supplied)
    native = RetrievalPolicy.model_validate((await service.app.tuning.profile(ctx))["value"])
    retrieval = TypeAdapter(JsonObject).validate_python(supplied["retrieval"])
    fusion = TypeAdapter(JsonObject).validate_python(retrieval.get("fusion", {}))
    unknown = (
        set(supplied).difference({"retrieval"})
        | set(retrieval).difference({"fusion"})
        | set(fusion).difference({"method", "rrf_k", "vector_weight", "bm25_weight"})
    )
    if unknown or fusion.get("method", "weighted_rrf") != "weighted_rrf":
        raise DomainError(
            "retrieval_profile_unsupported",
            "Select implemented Native retrieval parameters",
            fields=sorted(unknown),
        )
    updates = {
        target: fusion[source]
        for source, target in (
            ("rrf_k", "rrf_k"),
            ("vector_weight", "dense_weight"),
            ("bm25_weight", "lexical_weight"),
        )
        if source in fusion
    }
    return RetrievalPolicy.model_validate({**native.model_dump(mode="json"), **updates})


async def tuning_admin(
    service: AMemorixService, ctx: CallContext, arguments: JsonObject
) -> JsonObject:
    action = str(arguments.get("action", ""))
    owner = service.app.tuning
    operation = str(arguments.get("operation_id") or uuid4())
    if action in {"settings", "get_settings"}:
        return json_object(
            {
                "success": True,
                "settings": {
                    "default_rounds": CreateTuning.model_fields["max_trials"].default,
                    "request_schema": CreateTuning.model_json_schema(),
                },
            }
        )
    if action in {"get_profile", "export_profile"}:
        profile = await owner.profile(ctx)
        value = TypeAdapter(JsonObject).validate_python(profile["value"])
        return {
            "success": True,
            "profile": value,
            "runtime_profile": value,
            "persistable_profile": value,
            "commit_seq": profile["commit_seq"],
            "toml": tomlkit.dumps({"retrieval": value}),
        }
    if action == "apply_profile":
        profile = await profile_value(service, ctx, arguments.get("profile", {}))
        result = await owner.change_profile(
            ctx,
            ApplyRetrievalProfile(
                operation_id=operation,
                profile=profile,
                expected_commit_seq=TypeAdapter(int | None).validate_python(
                    arguments.get("expected_commit_seq")
                ),
            ),
        )
        return {"success": True, **result}
    if action == "rollback_profile":
        return {
            "success": True,
            **await owner.change_profile(
                ctx,
                RollbackRetrievalProfile(
                    operation_id=operation,
                    commit_seq=TypeAdapter(int | None).validate_python(arguments.get("commit_seq")),
                    expected_commit_seq=TypeAdapter(int | None).validate_python(
                        arguments.get("expected_commit_seq")
                    ),
                ),
            ),
        }
    if action == "create_task":
        payload = arguments.get("payload")
        values = TypeAdapter(JsonObject).validate_python(
            payload
            if payload is not None
            else {key: value for key, value in arguments.items() if key != "action"}
        )
        values.setdefault("operation_id", operation)
        if "cases" not in values:
            raise DomainError(
                "tuning_queries_required",
                "Supply existing queries and relevance judgments in cases",
            )
        return {
            "success": True,
            "task": task_value(await owner.create(ctx, CreateTuning.model_validate(values))),
        }
    if action == "list_tasks":
        page = await owner.list(
            ctx,
            limit=TypeAdapter(int).validate_python(arguments.get("limit", 50)),
            cursor=TypeAdapter(str | None).validate_python(arguments.get("cursor")),
        )
        return {
            "success": True,
            "items": [task_value(value) for value in page.items],
            "count": len(page.items),
            "next_cursor": page.next_cursor,
        }
    task_id = arguments.get("task_id")
    if not isinstance(task_id, str) or not task_id:
        raise DomainError("tuning_task_required", "Select a tuning task_id")
    if action in {"get_task", "cancel"}:
        value = await (
            owner.cancel(ctx, task_id) if action == "cancel" else owner.get(ctx, task_id)
        )
        result = task_value(value)
        if arguments.get("include_rounds") is True:
            result["rounds"] = [
                item.model_dump(mode="json")
                for item in (await owner.read_rounds(ctx, task_id)).items
            ]
        return {"success": True, "task": result}
    if action == "get_rounds":
        offset = TypeAdapter(int).validate_python(arguments.get("offset", 0))
        if offset < 0:
            raise ValueError("Round offset must be nonnegative")
        page = await owner.read_rounds(
            ctx,
            task_id,
            after=offset - 1,
            limit=TypeAdapter(int).validate_python(arguments.get("limit", 50)),
        )
        return json_object(
            {
                "success": True,
                "items": page.items,
                "rounds": page.items,
                "next_offset": page.next_cursor + 1 if page.next_cursor is not None else None,
            }
        )
    if action == "apply_best":
        return {
            "success": True,
            **await owner.change_profile(
                ctx,
                ApplyRetrievalProfile(
                    operation_id=operation,
                    task_id=task_id,
                    expected_commit_seq=TypeAdapter(int | None).validate_python(
                        arguments.get("expected_commit_seq")
                    ),
                ),
            ),
        }
    if action == "get_report":
        task = await owner.get(ctx, task_id)
        measured = (await owner.read_rounds(ctx, task_id)).items
        if arguments.get("format", "md") == "json":
            report: JsonValue = json_object({"task": task, "rounds": measured})
        else:
            lines = [
                "# Retrieval tuning",
                "",
                f"Task: {task.id}",
                f"Status: {task.status}",
                "",
                "| Trial | P@5 | Recall@10 | nDCG@10 | Mean ms | Failed queries |",
                "| --- | ---: | ---: | ---: | ---: | ---: |",
            ]
            for item in measured:
                m = item.metrics
                lines.append(
                    f"| {item.index} | {m.precision_at_5:.4f} | {m.recall_at_10:.4f} | {m.ndcg_at_10:.4f} | {m.mean_latency_ms:.2f} | {m.failed_queries} |"
                )
            lines.extend(
                ("", f"Selected trial: {task.best_index}", f"Holdout queries: {len(task.holdout)}")
            )
            report = "\n".join(lines)
        return {"success": True, "report": report}
    raise DomainError("action_invalid", "Unknown retrieval tuning action")
