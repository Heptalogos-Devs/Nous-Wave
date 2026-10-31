"""A-Memorix profiles through the formal domain owners."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.amemorix.contracts import (
    ProfileInput,
)
from nous_sdk.contracts import (
    CallContext,
)
from pydantic import JsonValue

from .helpers import json_object

if TYPE_CHECKING:
    from .service import AMemorixService


async def profile(
    self: AMemorixService, ctx: CallContext, command: ProfileInput
) -> dict[str, JsonValue]:
    from .profile_admin import profile_admin

    return await profile_admin(self, ctx, {**command.model_dump(mode="json"), "action": "query"})


async def stats(self: AMemorixService, ctx: CallContext) -> dict[str, JsonValue]:
    from .diagnostics import diagnostics

    observed = await diagnostics(self, ctx)
    counts = observed["cognition"]
    assert isinstance(counts, dict)
    retrieval = observed["retrieval"]
    assert isinstance(retrieval, dict)
    spaces = retrieval["spaces"]
    assert isinstance(spaces, list)
    vector_count = 0
    for space in spaces:
        if isinstance(space, dict):
            count = space.get("indexed_references")
            if isinstance(count, int):
                vector_count += count
    return json_object(
        {
            **observed,
            "success": True,
            "paragraphs": counts.get("memory", 0),
            "entities": counts.get("entity", 0),
            "relations": counts.get("association", 0),
            "episodes": counts.get("episode", 0),
            "vectors": vector_count,
        }
    )
