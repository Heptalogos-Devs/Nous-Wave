"""Public jobs use DBOS execution and the native source lifecycle."""

from dbos import WorkflowStatus
from nous_sdk.contracts import DomainError, SourceCommand
from nous_sdk.types import JsonObject
from pydantic import TypeAdapter

from nous.integrations.amemorix.sources import source_label
from nous.platform.execution.scope import JobPurpose

from .context import RPCContext, page_size, required_text, text_field
from .memory import timestamp


def job_info(value: WorkflowStatus, namespace_id: str) -> JsonObject:
    states = {
        "ENQUEUED": "PENDING",
        "DELAYED": "PENDING",
        "PENDING": "RUNNING",
        "SUCCESS": "SUCCEEDED",
        "CANCELLED": "CANCELLED",
        "ERROR": "FAILED",
        "MAX_RECOVERY_ATTEMPTS_EXCEEDED": "FAILED",
    }
    kind = (
        "JOB_TYPE_DELETE_BY_SOURCE"
        if value.name == "nous.sources.trash"
        else (
            "JOB_TYPE_RELATION_EXTRACTION"
            if value.name in {"nous.imports.process", "nous.imports.form_observation"}
            else "JOB_TYPE_UNSPECIFIED"
        )
    )
    result: JsonObject = {
        "job_id": value.workflow_id,
        "namespace_id": namespace_id,
        "job_type": kind,
        "status": "JOB_STATUS_" + states[value.status],
        "progress": 1 if value.status == "SUCCESS" else 0,
    }
    for name, milliseconds in (
        ("created_at", value.created_at),
        ("updated_at", value.updated_at),
        ("started_at", value.dequeued_at),
        ("completed_at", value.completed_at),
    ):
        if milliseconds is not None:
            result[name] = timestamp(milliseconds)
    if value.status == "SUCCESS" and value.output is not None:
        output: object = value.output
        result["result"] = (
            TypeAdapter(JsonObject).validate_python(output)
            if isinstance(output, dict)
            else {"value": TypeAdapter(object).dump_python(output, mode="json")}
        )
    if value.error is not None:
        result["error"] = {"code": "job_failed", "message": str(value.error)}
    return result


async def execute(method: str, values: JsonObject, rpc: RPCContext) -> JsonObject:
    if method == "SubmitDeleteBySource":
        from .context import object_field

        namespace_id = required_text(object_field(values, "context"), "namespace_id")
        ctx = await rpc.scope(namespace_id, "write", subject=True, request=values)
        label = required_text(values, "source")
        sources = await rpc.app.sources.list(ctx)
        ids = tuple(
            item["id"]
            for item in sources
            if source_label(TypeAdapter(JsonObject).validate_python(item)) == label
            and item["active"]
        )
        if not ids:
            raise DomainError("source_not_found", "Source name has no active observations")
        identity = await rpc.app.sources.submit_trash(
            ctx,
            SourceCommand(
                operation_id=rpc.operation_id(values, method),
                observation_ids=ids,
                reason=text_field(values, "reason") or "Delete by source",
            ),
        )
        return {"job": job_info(await rpc.app.execution.status(ctx, identity), namespace_id)}
    namespace_id = required_text(values, "namespace_id")
    management = "manage" in rpc.actor.capabilities
    purpose: JobPurpose = "management" if management else "actor"
    ctx = await rpc.scope(
        namespace_id, "manage" if management else ("write" if method == "CancelJob" else "read")
    )
    if method == "ListJobs":
        page = await rpc.app.execution.query_jobs(
            ctx,
            purpose=purpose,
            limit=page_size(values),
            cursor=text_field(values, "page_token") or None,
        )
        return {
            "jobs": [job_info(item, namespace_id) for item in page.items],
            "next_page_token": page.next_cursor or "",
        }
    identity = required_text(values, "job_id")
    if method == "CancelJob":
        await rpc.app.execution.cancel(ctx, identity, purpose=purpose)
    elif method != "GetJob":
        raise DomainError("rpc_method_not_found", "Unknown job operation")
    return {
        "job": job_info(
            await rpc.app.execution.status(ctx, identity, purpose=purpose), namespace_id
        )
    }
