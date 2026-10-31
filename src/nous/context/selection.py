"""Automatic residency respects current activation while retaining exact source identity."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, CognitiveRecord, CognitiveRef
from nous_sdk.contracts.context import CognitiveTarget, ContextSegment

from nous.platform.persistence import ReadView, WriteTx

if TYPE_CHECKING:
    from .managed import ManagedContexts


async def automatic_resident_in(
    self: ManagedContexts,
    view: ReadView | WriteTx,
    ctx: CallContext,
    reference: CognitiveRef,
) -> CognitiveRecord | None:
    return await self.cognition.automatic_in(view, ctx, reference)


async def segment_eligible_in(
    self: ManagedContexts,
    view: ReadView | WriteTx,
    ctx: CallContext,
    segment: ContextSegment,
) -> bool:
    await self.require_targets_in(view, ctx, segment.sources)
    if not segment.required:
        for target in segment.sources:
            if (
                isinstance(target, CognitiveTarget)
                and await automatic_resident_in(self, view, ctx, target.reference) is None
            ):
                return False
    return True
