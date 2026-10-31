from fastapi import APIRouter, Request
from nous_sdk.contracts.vector_spaces import (
    CheckVectorSpaces,
    DeleteVectorSpace,
    RebuildVectors,
    RecoverVectors,
    RetryVectorProjection,
)

from .security import application, context

router = APIRouter(prefix="/api/namespaces/{namespace_id}")


@router.get("/vector-spaces")
async def namespace_spaces(request: Request, namespace_id: str):
    return await application(request).vector_spaces.catalog(
        await context(request, namespace_id, capability="manage")
    )


@router.post("/vector-spaces/delete")
async def delete_space(request: Request, namespace_id: str, command: DeleteVectorSpace):
    return await application(request).vector_spaces.delete(
        await context(request, namespace_id, capability="manage"), command
    )


@router.get("/subjects/{subject_id}/vector-spaces")
async def spaces(request: Request, namespace_id: str, subject_id: str, include_media: bool = False):
    return await application(request).vector_spaces.catalog(
        await context(request, namespace_id, subject_id, capability="read"),
        include_media=include_media,
    )


@router.post("/subjects/{subject_id}/vectors/rebuild")
async def rebuild(request: Request, namespace_id: str, subject_id: str, command: RebuildVectors):
    return await application(request).vector_spaces.rebuild(
        await context(request, namespace_id, subject_id, capability="manage"), command
    )


@router.post("/subjects/{subject_id}/vectors/check")
async def check(request: Request, namespace_id: str, subject_id: str, command: CheckVectorSpaces):
    return await application(request).vector_spaces.check(
        await context(
            request, namespace_id, subject_id, capability="write" if command.probe else "read"
        ),
        command,
    )


@router.get("/subjects/{subject_id}/vectors/projections/{request_id}")
async def progress(request: Request, namespace_id: str, subject_id: str, request_id: str):
    return await application(request).vector_spaces.progress(
        await context(request, namespace_id, subject_id, capability="read"), request_id
    )


@router.post("/subjects/{subject_id}/vectors/retry")
async def retry(
    request: Request, namespace_id: str, subject_id: str, command: RetryVectorProjection
):
    return await application(request).vector_spaces.retry(
        await context(request, namespace_id, subject_id), command
    )


@router.post("/subjects/{subject_id}/vectors/recover")
async def recover(request: Request, namespace_id: str, subject_id: str, command: RecoverVectors):
    return await application(request).vector_spaces.recover(
        await context(request, namespace_id, subject_id, capability="manage"), command
    )


@router.get("/persistence")
@router.get("/subjects/{subject_id}/vectors/persistence")
async def persistence(request: Request, namespace_id: str, subject_id: str | None = None):
    return await application(request).vector_spaces.persistence(
        await context(request, namespace_id, subject_id, capability="read")
    )
