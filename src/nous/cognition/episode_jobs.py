"""Episode job controls reuse the single execution directory, wait and resume owners."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext
from nous_sdk.contracts.episodes import ProcessEpisodeSources
from pydantic import JsonValue

if TYPE_CHECKING:
    from .episodes import EpisodeService


async def status(self: EpisodeService, ctx: CallContext, limit: int = 20) -> dict[str, JsonValue]:
    return await self.execution.summarize_jobs(ctx, kind="nous.episodes.rebuild", limit=limit)


async def process_sources(
    self: EpisodeService, ctx: CallContext, command: ProcessEpisodeSources
) -> dict[str, JsonValue]:
    batch = await self.execution.process_jobs(
        ctx, kind="nous.episodes.rebuild", limit=command.limit, max_retry=command.max_retry
    )
    completed = batch["items"]
    assert isinstance(completed, list)
    results = [item["result"] for item in completed if isinstance(item, dict)]
    rebuilt = sum(
        count
        for result in results
        if isinstance(result, dict) and isinstance(count := result.get("rebuilt"), int)
    )
    return {
        "processed": batch["processed"],
        "unfinished": batch["unfinished"],
        "rebuilt": rebuilt,
        "failed": batch["failed"],
        "failures": batch["failures"],
        "results": results,
    }
