"""Commit extracted text through the same exact derived-media owner as model outputs."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import CallContext, MediaReference
from nous_sdk.contracts.derivation import (
    CreateDerivedRegion,
    CreateRepresentation,
    DerivationInput,
    DerivationProducer,
    DerivedSelector,
    TextRepresentation,
)

from nous.material.derived_producer import digest
from nous.platform.persistence import WriteTx

if TYPE_CHECKING:
    from .service import ImportService


async def adopt_extracted_in(
    self: ImportService,
    tx: WriteTx,
    ctx: CallContext,
    original: MediaReference,
    text: str,
    start: int,
    end: int,
    implementation: str,
) -> MediaReference:
    config = {
        "implementation": implementation,
        "tables": True,
        "source": original.model_dump(mode="json"),
    }
    representation = await self.cognition.derivations.adopt_in(
        tx,
        ctx,
        CreateRepresentation(
            inputs=(DerivationInput(ordinal=0, source=original),),
            kind="extracted_text",
            strategy="preprocess",
            payload=TextRepresentation(text=text),
            producer=DerivationProducer(
                implementation=implementation,
                signature=digest((config, text)),
                config_digest=digest(config),
            ),
        ),
    )
    region = await self.cognition.derivations.adopt_region_in(
        tx,
        ctx,
        CreateDerivedRegion(
            representation=representation.reference,
            selector=DerivedSelector(kind="text_span", start=start, end=end),
        ),
    )
    return MediaReference(kind="derived_region", id=region.id)
