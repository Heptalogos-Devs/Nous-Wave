"""Runtime administration reads and changes the configuration owner's records."""

from typing import TYPE_CHECKING, Literal

from nous_sdk.contracts import CallContext, CognitiveRef, Contract
from nous_sdk.contracts.cognitive_identity import CognitiveKind
from nous_sdk.contracts.vector_spaces import (
    CheckVectorSpaces,
    DeleteVectorSpace,
    RebuildVectors,
    RecoverVectors,
    RetryVectorProjection,
)
from pydantic import JsonValue, TypeAdapter

from .helpers import json_object

if TYPE_CHECKING:
    from .service import AMemorixService


class RuntimeInput(Contract):
    action: Literal[
        "get_config",
        "list_vector_spaces",
        "save",
        "self_check",
        "refresh_self_check",
        "recover_embedding",
        "rebuild_all_vectors",
        "paragraph_backfill_once",
        "set_auto_save",
        "delete_vector_space",
        "projection_progress",
        "retry_projection",
    ]
    operation_id: str | None = None
    enabled: bool = True
    probe: bool = False
    sample_text: str | None = None
    sample_reference: CognitiveRef | None = None
    include_relations: bool | None = None
    batch_size: int | None = None
    limit: int | None = None
    max_retry: int | None = None
    dry_run: bool = False
    space_id: str = ""
    request_id: str = ""


async def runtime_admin(
    service: AMemorixService, ctx: CallContext, arguments: dict[str, JsonValue]
) -> dict[str, JsonValue]:
    command = RuntimeInput.model_validate(arguments)
    if command.action in {"get_config", "list_vector_spaces"}:
        from .diagnostics import diagnostics

        return await diagnostics(service, ctx)
    owner = service.app.vector_spaces
    identity = command.operation_id or "amemorix-runtime:" + ctx.request_id
    if command.action in {"save", "set_auto_save"}:
        state = await owner.persistence(ctx)
        if command.action == "set_auto_save" and not command.enabled:
            return json_object(
                {
                    "success": False,
                    "auto_save": True,
                    "reason": "transaction_commit_required",
                    "persistence": state,
                }
            )
        return json_object(
            {
                "success": True,
                "saved": command.action == "save",
                "auto_save": True,
                "persistence": state,
            }
        )
    if command.action in {"self_check", "refresh_self_check"}:
        report = await owner.check(
            ctx,
            CheckVectorSpaces(
                operation_id=identity,
                probe=command.probe,
                sample_text=command.sample_text,
                sample_reference=command.sample_reference,
            ),
        )
        return json_object(
            {
                "success": not report.invalid_references
                and report.configured
                and report.credential_ready,
                "report": report,
            }
        )
    if command.action == "delete_vector_space":
        return json_object(
            {
                "success": True,
                "result": await owner.delete(
                    ctx, DeleteVectorSpace(operation_id=identity, space_id=command.space_id)
                ),
            }
        )
    if command.action == "projection_progress":
        return json_object(
            {"success": True, "progress": await owner.progress(ctx, command.request_id)}
        )
    if command.action == "retry_projection":
        return json_object(
            {
                "success": True,
                "projection": await owner.retry(
                    ctx, RetryVectorProjection(operation_id=identity, request_id=command.request_id)
                ),
            }
        )
    kinds: tuple[CognitiveKind, ...] = (
        ("memory",) if command.action == "paragraph_backfill_once" else ()
    )
    if command.include_relations is False and not kinds:
        from typing import get_args

        kinds = TypeAdapter(tuple[CognitiveKind, ...]).validate_python(
            tuple(kind for kind in get_args(CognitiveKind) if kind != "association")
        )
    values: dict[str, object] = {"operation_id": identity, "kinds": kinds}
    if command.limit is not None:
        values["limit"] = command.limit
    if command.action in {"recover_embedding", "paragraph_backfill_once"}:
        if command.max_retry is not None:
            values["max_retry"] = command.max_retry
        recovered = await owner.recover(ctx, RecoverVectors.model_validate(values))
        catalog = await owner.catalog(ctx)
        receipts = (
            *recovered.resumed,
            *((recovered.backfill.projection,) if recovered.backfill.projection else ()),
        )
        return json_object(
            {
                "success": not recovered.frozen and not recovered.backfill.blocked_references,
                "recovered": catalog.configured_space_complete is True,
                "result": recovered,
                "job_ids": tuple(receipt.job_id for receipt in receipts),
                "queued": bool(receipts),
            }
        )
    result = await owner.rebuild(
        ctx,
        RebuildVectors.model_validate(
            {**values, "mode": "all", "batch_size": command.batch_size, "dry_run": command.dry_run}
        ),
    )
    return json_object(
        {
            "success": not result.blocked_references,
            "queued": result.projection is not None,
            "result": result,
            "job_id": result.projection.job_id if result.projection else None,
            "request_id": result.projection.request_id if result.projection else None,
        }
    )
