"""Protocol dispatch owns authentication-derived request admission, without domain state."""

from nous_sdk.contracts import DomainError
from nous_sdk.types import JsonObject

from . import auth, backups, jobs, memory, namespaces
from .context import RPCContext, object_field, text_field

_HANDLERS = {
    "AuthService": auth.execute,
    "NamespaceService": namespaces.execute,
    "MemoryService": memory.execute,
    "JobService": jobs.execute,
    "BackupService": backups.execute,
}


async def dispatch(service: str, method: str, values: JsonObject, rpc: RPCContext) -> JsonObject:
    handler = _HANDLERS.get(service)
    if handler is None:
        raise DomainError("rpc_service_not_found", "Unknown RPC service")
    namespace_id = text_field(values, "namespace_id") or text_field(
        object_field(values, "context"), "namespace_id"
    )
    if namespace_id and service != "BackupService":
        capability = "read" if service != "AuthService" else "manage"
        # Namespace-only keys and native subject-scoped keys both bind to the
        # configured memory subject. The caller supplied agent_id grants no scope.
        ctx = await rpc.scope(
            namespace_id,
            capability,
            subject=service == "MemoryService"
            or (service == "JobService" and method == "SubmitDeleteBySource"),
            request=values,
        )
        async with rpc.app.namespace_limits.request(ctx):
            return await handler(method, values, rpc)
    return await handler(method, values, rpc)
