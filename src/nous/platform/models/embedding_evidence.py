"""Reuse already paid vectors only when their actual invocation proves the input."""

from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, DomainError, MediaEmbeddingInput

from nous.platform.persistence import ReadView, WriteTx

from .embedding import retained_embedding
from .embedding_profile import EmbeddingEvidence, EmbeddingProfile

if TYPE_CHECKING:
    from . import ModelService


async def reusable_embedding_in(
    self: ModelService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    call_id: str,
    profile: EmbeddingProfile,
    *,
    text: str | None,
    media: MediaEmbeddingInput | None,
    source_ids: tuple[str, ...],
) -> EmbeddingEvidence | None:
    try:
        paid = await self.ledger.successful_in(view, ctx, call_id)
    except DomainError as error:
        if error.code in {"model_call_unavailable", "model_call_forbidden"}:
            return None
        raise
    request = paid.request
    route = next(
        (
            route
            for route in (profile.route, *profile.fallbacks)
            if EmbeddingProfile.matches_request(route, request)
        ),
        None,
    )
    if route is None:
        return None
    recorded = request.get("source_ids")
    if not isinstance(recorded, list) or any(not isinstance(item, str) for item in recorded):
        return None
    sources = tuple(item for item in recorded if isinstance(item, str))
    if not set(source_ids).issubset(sources):
        return None
    index: int | None = None
    if text is not None:
        texts = request.get("texts")
        if isinstance(texts, list):
            index = next((i for i, value in enumerate(texts) if value == text), None)
    elif media is not None and request.get("path") == route.media_path:
        if request.get("parts") == [media.model_dump(mode="json")]:
            index = 0
    if index is None:
        return None
    batch = retained_embedding(paid.result, paid.origin_call_id or paid.id)
    if index >= len(batch.vectors):
        raise DomainError("invalid_embedding", "Retained embedding input and vector counts differ")
    return EmbeddingEvidence(
        batch=batch.model_copy(
            update={
                "vectors": (batch.vectors[index],),
                "call_ids": (call_id,),
            }
        ),
        source_ids=sources,
    )


async def find_embedding_in(
    self: ModelService,
    view: ReadView | WriteTx,
    ctx: CallContext,
    profile: EmbeddingProfile,
    *,
    text: str | None,
    media: MediaEmbeddingInput | None,
    source_ids: tuple[str, ...],
) -> EmbeddingEvidence | None:
    blocked: DomainError | None = None
    for call_id in await self.ledger.embedding_candidates_in(view, ctx, text):
        try:
            records = await self.ledger.export_in(view, ctx, (call_id,))
        except DomainError as error:
            if error.code in {"model_call_forbidden", "model_call_unavailable"}:
                continue
            raise
        actual = records[0]
        if actual.status != "succeeded":
            request = actual.request
            matching = any(
                all(
                    request.get(key) == value
                    for key, value in (
                        ("model", route.model),
                        ("base_url", route.base_url),
                        ("dimensions", route.dimensions),
                        ("normalization", route.normalization),
                        ("extra_body", route.extra_body),
                    )
                )
                and (text is not None or request.get("path") == route.media_path)
                for route in (profile.route, *profile.fallbacks)
            )
            recorded_sources = request.get("source_ids")
            matching = (
                matching
                and isinstance(recorded_sources, list)
                and set(source_ids).issubset(
                    item for item in recorded_sources if isinstance(item, str)
                )
            )
            if media is not None:
                matching = matching and request.get("parts") == [media.model_dump(mode="json")]
            usage = actual.usage or {}
            outcome = usage.get("outcome")
            safe = isinstance(outcome, dict) and outcome.get("kind") in {"not_sent", "rejected"}
            if matching and (actual.status == "reserved" or not safe):
                blocked = DomainError(
                    "model_invocation_unavailable",
                    "An earlier embedding of this exact input has an unresolved paid outcome",
                    call_id=call_id,
                    status=actual.status,
                    error_code=actual.error_code,
                )
            continue
        retained = await self.reusable_embedding_in(
            view, ctx, call_id, profile, text=text, media=media, source_ids=source_ids
        )
        if retained is not None:
            return retained
    if blocked is not None:
        raise blocked
    return None
