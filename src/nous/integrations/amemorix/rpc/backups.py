"""The public backup protocol transports native Namespace archives."""

import base64

from nous_sdk.contracts import DomainError
from nous_sdk.contracts.namespace_backups import RestoreNamespaceBackup
from nous_sdk.types import JsonObject

from .context import RPCContext, integer_field, page_size, required_text, text_field
from .namespaces import namespace_info


async def execute(method: str, values: JsonObject, rpc: RPCContext) -> JsonObject:
    rpc.administrator()
    service = rpc.app.namespace_backups
    if method == "BeginNamespaceBackupUpload":
        ctx = await rpc.scope("instance", "manage")
        return (await service.begin_upload(ctx)).model_dump(mode="json")
    if method in {
        "UploadNamespaceBackupChunk",
        "CompleteNamespaceBackupUpload",
        "AbortNamespaceBackupUpload",
    }:
        upload_id = required_text(values, "upload_id")
        namespace_id = await service.namespace_for_upload(upload_id)
        ctx = await rpc.scope(namespace_id, "manage")
        if method == "UploadNamespaceBackupChunk":
            data = base64.b64decode(text_field(values, "data"), validate=True)
            return (
                await service.upload_chunk(ctx, upload_id, integer_field(values, "offset"), data)
            ).model_dump(mode="json")
        if method == "CompleteNamespaceBackupUpload":
            backup = await service.complete_upload(
                ctx, upload_id, expected_sha256=text_field(values, "expected_sha256")
            )
            return {"backup": backup.model_dump(mode="json")}
        await service.abort_upload(ctx, upload_id)
        return {}
    if method == "ListNamespaceBackups":
        namespace_id = text_field(values, "source_namespace_id")
        ctx = await rpc.scope(namespace_id or "instance", "manage")
        listing = service.list if namespace_id else service.list_all
        page = await listing(
            ctx, limit=page_size(values), cursor=text_field(values, "page_token") or None
        )
        return {
            "backups": [item.model_dump(mode="json") for item in page.items],
            "next_page_token": page.next_cursor or "",
        }
    if method == "CreateNamespaceBackup":
        namespace_id = required_text(values, "namespace_id")
        ctx = await rpc.scope(namespace_id, "manage")
        async with rpc.app.namespace_limits.request(ctx):
            return {"backup": (await service.create(ctx)).model_dump(mode="json")}
    backup_id = required_text(values, "backup_id")
    namespace_id = await service.namespace_for_backup(backup_id)
    ctx = await rpc.scope(namespace_id, "manage")
    if method == "GetNamespaceBackup":
        return {"backup": (await service.get(ctx, backup_id)).model_dump(mode="json")}
    if method == "DeleteNamespaceBackup":
        await service.delete(ctx, backup_id)
        return {}
    if method == "DownloadNamespaceBackup":
        chunk = await service.download(
            ctx,
            backup_id,
            offset=integer_field(values, "offset"),
            max_bytes=integer_field(values, "max_bytes", 262144),
        )
        return {
            "backup": chunk.backup.model_dump(mode="json"),
            "offset": chunk.offset,
            "data": chunk.data_base64,
            "next_offset": chunk.next_offset,
            "complete": chunk.complete,
        }
    if method == "RestoreNamespaceFromBackup":
        target = required_text(values, "target_namespace_id")
        ctx = await rpc.scope(target, "manage")
        restored = await service.restore(
            ctx,
            RestoreNamespaceBackup(
                backup_id=backup_id, operation_id=rpc.operation_id(values, method)
            ),
        )
        return {"namespace": await namespace_info(rpc, restored.namespace)}
    raise DomainError("rpc_method_not_found", "Unknown backup operation")
