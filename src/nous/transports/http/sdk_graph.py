"""Canonical graph management shares source-backed cognitive transactions."""

from fastapi import APIRouter, Request
from nous_sdk.contracts.graph_management import (
    GraphChange,
    GraphChangeResult,
    GraphReadQuery,
    GraphSearchQuery,
    GraphSearchResult,
    GraphSnapshot,
)

from .security import application, context

router = APIRouter()


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/graph/query")
async def read(
    request: Request, namespace_id: str, subject_id: str, command: GraphReadQuery
) -> GraphSnapshot:
    return await application(request).cognition.read_graph(
        await context(request, namespace_id, subject_id, capability="read"), command
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/graph/search")
async def search(
    request: Request, namespace_id: str, subject_id: str, command: GraphSearchQuery
) -> GraphSearchResult:
    return await application(request).cognition.search_graph(
        await context(request, namespace_id, subject_id, capability="read"), command
    )


@router.get("/namespaces/{namespace_id}/subjects/{subject_id}/graph")
async def snapshot(request: Request, namespace_id: str, subject_id: str) -> GraphSnapshot:
    return await application(request).cognition.graph_snapshot(
        await context(request, namespace_id, subject_id, capability="read")
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/graph")
async def change(
    request: Request, namespace_id: str, subject_id: str, command: GraphChange
) -> GraphChangeResult:
    return await application(request).cognition.manage_graph(
        await context(request, namespace_id, subject_id), command
    )
