"""The graph directory is a view of authorized canonical revisions."""

from __future__ import annotations

from typing import TYPE_CHECKING

from nous_sdk.contracts import (
    AssociationBody,
    CallContext,
    CognitiveRecord,
    CognitiveRef,
    DomainError,
    EntityBody,
    MemoryBody,
)
from nous_sdk.contracts.graph_management import (
    GraphReadQuery,
    GraphSearchMatch,
    GraphSearchQuery,
    GraphSearchResult,
    GraphSnapshot,
)
from sqlalchemy import select

from nous.identity.access import require_subject_in
from nous.platform.persistence import ReadView, WriteTx

from .lifecycle import selectable
from .projection import references_in
from .schema import revisions

if TYPE_CHECKING:
    from .service import CognitionService


async def heads_in(
    view: ReadView | WriteTx,
    ctx: CallContext,
    kinds: tuple[str, ...] = ("entity", "association"),
    *,
    reference_ids: tuple[str, ...] | None = None,
) -> tuple[CognitiveRecord, ...]:
    statement = select(revisions.c.record).where(
        revisions.c.namespace_id == ctx.namespace_id,
        revisions.c.subject_id == ctx.require_subject(),
        revisions.c.head.is_(True),
        revisions.c.kind.in_(kinds),
    )
    if reference_ids is not None:
        statement = statement.where(revisions.c.id.in_(reference_ids))
    values = (await view.connection.execute(statement.order_by(revisions.c.id))).scalars()
    return tuple(CognitiveRecord.model_validate(value) for value in values)


def resolve_entity(
    records: tuple[CognitiveRecord, ...], token: str, *, include_deleted: bool = False
) -> CognitiveRecord | None:
    key = token.strip().casefold()
    if not key:
        return None
    matches = [
        record
        for record in records
        if isinstance(record.body, EntityBody)
        and selectable(record, include_deleted=include_deleted)
        and (
            token
            in {record.reference.id, record.reference.exact(), *record.body.identifiers.values()}
            or key
            in {value.casefold() for value in (record.body.label, *record.body.effective_aliases)}
        )
    ]
    if len(matches) > 1:
        raise DomainError("ambiguous_entity", "The selector identifies multiple Entity identities")
    return matches[0] if matches else None


async def graph_snapshot(self: CognitionService, ctx: CallContext) -> GraphSnapshot:
    async with self.authority.read(ctx) as view:
        await require_subject_in(view, ctx)
        records: list[CognitiveRecord] = []
        paragraphs: dict[str, CognitiveRecord] = {}
        for record in await heads_in(view, ctx):
            record = await self.project_record_in(view, ctx, record)
            if not selectable(record) or not await self.visible_in(view, ctx, record):
                continue
            records.append(record)
            references = tuple(references_in(record.body)) + tuple(
                link.reference for link in record.evidence if link.reference
            )
            for reference in references:
                if reference.kind != "memory" or reference.exact() in paragraphs:
                    continue
                try:
                    value = await self.automatic_in(view, ctx, reference)
                except DomainError as error:
                    if error.code in {"source_unavailable", "cognition_not_found"}:
                        continue
                    raise
                if value is not None and isinstance(value.body, MemoryBody):
                    paragraphs[reference.exact()] = value
        return GraphSnapshot(
            records=tuple(records), paragraphs=tuple(paragraphs.values()), watermark=view.commit_seq
        )


async def search_graph(
    self: CognitionService, ctx: CallContext, command: GraphSearchQuery
) -> GraphSearchResult:
    query = command.text.strip().casefold()
    if not query:
        raise DomainError("invalid_query", "Graph search text must not be empty")
    snapshot = await self.graph_snapshot(ctx)
    known = {record.reference.exact(): record for record in snapshot.records}

    async def name(reference: CognitiveRef) -> str:
        record = known.get(reference.exact())
        if record is None:
            record = await self.get(ctx, reference)
            known[reference.exact()] = record
        return record.body.label if isinstance(record.body, EntityBody) else reference.exact()

    matches: list[GraphSearchMatch] = []
    for record in snapshot.records:
        body = record.body
        source_name = target_name = None
        fields: tuple[tuple[str, str], ...]
        if isinstance(body, EntityBody):
            fields = (("name", body.label), ("id", record.reference.id))
        elif isinstance(body, AssociationBody):
            try:
                source_name, target_name = await name(body.from_ref), await name(body.to_ref)
            except DomainError as error:
                if error.code not in {"source_unavailable", "cognition_not_found"}:
                    raise
                continue
            fields = (
                ("subject", source_name),
                ("object", target_name),
                ("predicate", body.relation),
                ("id", record.reference.id),
            )
        else:
            continue
        found: tuple[str, str, int] | None = None
        for field, value in fields:
            token = value.strip().casefold()
            rank = (
                0
                if token == query
                else 1
                if token.startswith(query)
                else 2
                if query in token
                else None
            )
            if token and rank is not None and (found is None or rank < found[2]):
                found = (field, value, rank)
        if found:
            matches.append(
                GraphSearchMatch(
                    record=record,
                    field=found[0],
                    value=found[1],
                    rank=found[2],
                    source_name=source_name,
                    target_name=target_name,
                )
            )
    matches.sort(
        key=lambda item: (
            item.rank,
            item.record.reference.kind != "entity",
            (item.source_name or item.value).casefold(),
            item.record.reference.id,
        )
    )
    selected = matches[: command.limit]
    for item in selected:
        await self.get(ctx, item.record.reference)
    return GraphSearchResult(
        query=command.text.strip(),
        matches=tuple(selected),
        total_matches=len(matches),
        watermark=snapshot.watermark,
    )


async def read_graph(
    self: CognitionService, ctx: CallContext, command: GraphReadQuery
) -> GraphSnapshot:
    snapshot = await self.graph_snapshot(ctx)
    if not (command.node or command.source or command.target):
        return snapshot

    def resolve(token: str | None) -> str | None:
        if token is None:
            return None
        record = resolve_entity(snapshot.records, token)
        if record is None:
            raise DomainError("entity_not_found", "The selected graph Entity is unavailable")
        return record.reference.id

    node, source, target = resolve(command.node), resolve(command.source), resolve(command.target)
    selected: list[CognitiveRecord] = []
    identities = {value for value in (node, source, target) if value is not None}
    for record in snapshot.records:
        body = record.body
        if (
            isinstance(body, AssociationBody)
            and (node is None or node in {body.from_ref.id, body.to_ref.id})
            and (source is None or source == body.from_ref.id)
            and (target is None or target == body.to_ref.id)
        ):
            selected.append(record)
            identities.update((body.from_ref.id, body.to_ref.id))
    selected.extend(
        record
        for record in snapshot.records
        if isinstance(record.body, EntityBody) and record.reference.id in identities
    )
    evidence = {
        reference.id
        for record in selected
        for reference in (
            *references_in(record.body),
            *(link.reference for link in record.evidence if link.reference),
        )
    }
    return GraphSnapshot(
        records=tuple(selected),
        paragraphs=tuple(
            record for record in snapshot.paragraphs if record.reference.id in evidence
        ),
        watermark=snapshot.watermark,
    )
