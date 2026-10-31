"""Public API-key fields project the native credential authority."""

from __future__ import annotations

from nous_sdk.contracts import DomainError
from nous_sdk.contracts.credentials import CreateCredential, CredentialRecord
from nous_sdk.types import JsonObject

from .context import RPCContext, encoded, page_size, required_text, text_field


def key_info(record: CredentialRecord, namespace_id: str) -> JsonObject:
    result: JsonObject = {
        "key_id": record.id,
        "namespace_id": namespace_id,
        "label": record.name,
        "created_at": record.created_at,
    }
    if record.expires_at is not None:
        result["expires_at"] = encoded(record.expires_at)
    if record.revoked_at is not None:
        result["revoked_at"] = record.revoked_at
    return result


async def execute(method: str, values: JsonObject, rpc: RPCContext) -> JsonObject:
    rpc.administrator()
    namespace_id = required_text(values, "namespace_id")
    ctx = await rpc.scope(namespace_id, "manage")
    await rpc.app.identity.namespace(ctx, purpose="management")
    if method == "CreateApiKey":
        command = CreateCredential.model_validate(
            {
                "name": text_field(values, "label") or "A-Memorix API key",
                "namespace_ids": (namespace_id,),
                "capabilities": ("read", "write"),
                **({"expires_at": values["expires_at"]} if "expires_at" in values else {}),
            }
        )
        issued = await rpc.app.credentials.create(rpc.actor, command)
        return {"api_key": key_info(issued.record, namespace_id), "secret": issued.token or ""}
    if method == "ListApiKeys":
        records = [
            record
            for record in await rpc.app.credentials.list(rpc.actor)
            if record.namespace_ids == (namespace_id,)
            and record.subject_ids is None
            and record.source_ids is None
        ]
        records.sort(key=lambda record: record.id)
        cursor = text_field(values, "page_token")
        if cursor:
            records = [record for record in records if record.id > cursor]
        maximum = page_size(values)
        return {
            "api_keys": [key_info(record, namespace_id) for record in records[:maximum]],
            "next_page_token": records[maximum - 1].id if len(records) > maximum else "",
        }
    if method == "RevokeApiKey":
        identity = required_text(values, "key_id")
        record = next(
            (
                item
                for item in await rpc.app.credentials.list(rpc.actor)
                if item.id == identity and item.namespace_ids == (namespace_id,)
            ),
            None,
        )
        if record is None:
            raise DomainError("credential_not_found", "API key is unavailable in this namespace")
        await rpc.app.credentials.revoke(
            rpc.actor,
            identity,
            rpc.operation_id(values, "RevokeApiKey"),
            reason="A-Memorix RPC credential revocation",
        )
        return {}
    raise DomainError("rpc_method_not_found", "Unknown authentication operation")
