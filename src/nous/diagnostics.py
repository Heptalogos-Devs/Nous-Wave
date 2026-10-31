"""Authorized subject inventories composed from the native domain owners."""

from collections import Counter
from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, CognitiveKind
from nous_sdk.contracts.diagnostics import SubjectDiagnostics
from nous_sdk.contracts.records import RecordsQuery
from pydantic import JsonValue, TypeAdapter

from nous.identity.access import require_subject_in

if TYPE_CHECKING:
    from nous.application import Application


class SubjectDiagnosticsService:
    def __init__(self, app: Application) -> None:
        self.app = app

    async def read(self, ctx: CallContext) -> SubjectDiagnostics:
        counts: Counter[CognitiveKind] = Counter()
        async with self.app.authority.read(ctx) as view:
            await require_subject_in(view, ctx)
            command = RecordsQuery(limit=1000)
            while True:
                page = await self.app.cognition.records_in(view, ctx, command)
                counts.update(entry.record.reference.kind for entry in page.items)
                if page.next_cursor is None:
                    break
                command = command.model_copy(update={"cursor": page.next_cursor})
            watermark = view.commit_seq
        jobs = await self.app.execution.list_jobs(ctx)
        calls = await self.app.models.ledger.list(ctx)
        usage: dict[str, int] = {}
        for call in calls:
            measured = TypeAdapter(JsonValue).validate_python(call.get("usage"))
            if isinstance(measured, dict):
                for key in ("input_tokens", "output_tokens", "total_tokens"):
                    value = measured.get(key)
                    if isinstance(value, int):
                        usage[key] = usage.get(key, 0) + value
        return SubjectDiagnostics(
            namespace_id=ctx.namespace_id,
            subject_id=ctx.require_subject(),
            authority_commit_seq=watermark,
            cognition=dict(counts),
            tasks=dict(Counter(job.status for job in jobs)),
            retrieval=await self.app.retrieval.status(ctx),
            imports=await self.app.imports.vector_status(ctx),
            models=TypeAdapter(dict[str, JsonValue]).validate_python(
                await self.app.models.status(ctx)
            ),
            model_calls=len(calls),
            model_usage=usage,
        )
