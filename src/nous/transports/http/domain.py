"""Thin typed HTTP entrypoints into the shared domain owners."""

from fastapi import APIRouter, Request
from nous_sdk.contracts import (
    AdmitFiles,
    AdoptCognition,
    FreezeCatalog,
    ProcessCatalog,
    QueryRequest,
)

from .identities import router as identities_router
from .namespace_backups import router as namespace_backups_router
from .security import application, context
from .tuning import router as tuning_router

router = APIRouter(prefix="/api")

router.include_router(identities_router)
router.include_router(namespace_backups_router)
router.include_router(tuning_router)


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/imports/catalogs")
async def freeze(request: Request, namespace_id: str, subject_id: str, command: FreezeCatalog):
    return await application(request).imports.freeze_catalog(
        await context(request, namespace_id, subject_id), command
    )


@router.get("/namespaces/{namespace_id}/subjects/{subject_id}/imports/catalogs/{catalog_id}")
async def catalog(request: Request, namespace_id: str, subject_id: str, catalog_id: str):
    return await application(request).imports.read_catalog(
        await context(request, namespace_id, subject_id, capability="read"), catalog_id
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/imports/process")
async def process(request: Request, namespace_id: str, subject_id: str, command: ProcessCatalog):
    return await application(request).imports.submit_process(
        await context(request, namespace_id, subject_id), command
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/imports/files")
async def files(request: Request, namespace_id: str, subject_id: str, command: AdmitFiles):
    return await application(request).imports.admit_files(
        await context(request, namespace_id, subject_id), command
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/cognition")
async def adopt(request: Request, namespace_id: str, subject_id: str, command: AdoptCognition):
    return await application(request).cognition.adopt(
        await context(request, namespace_id, subject_id), command
    )


@router.post("/namespaces/{namespace_id}/subjects/{subject_id}/query")
async def query(request: Request, namespace_id: str, subject_id: str, command: QueryRequest):
    return await application(request).retrieval.query(
        await context(request, namespace_id, subject_id, capability="read"), command
    )


@router.get("/namespaces/{namespace_id}/subjects/{subject_id}/materials/{material_id}")
async def material(request: Request, namespace_id: str, subject_id: str, material_id: str):
    return await application(request).material.read(
        await context(request, namespace_id, subject_id, capability="read"), material_id
    )
