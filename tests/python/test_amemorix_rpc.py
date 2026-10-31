"""The published A-Memorix protobuf clients operate the native Nous application."""

# ruff: noqa: E402

import asyncio
from pathlib import Path

import pytest

grpc = pytest.importorskip("grpc")

from nous.bootstrap import application
from nous.integrations.amemorix.rpc._protocol import (
    auth_pb2,
    auth_pb2_grpc,
    backup_pb2,
    backup_pb2_grpc,
    common_pb2,
    job_pb2,
    job_pb2_grpc,
    memory_pb2,
    memory_pb2_grpc,
    namespace_pb2,
    namespace_pb2_grpc,
)
from nous.integrations.amemorix.rpc.dispatch import dispatch
from nous.integrations.amemorix.rpc.server import RPCServer
from nous.platform.configuration import Settings


def test_wire_memory_scope_jobs_and_namespace_backup(tmp_path):
    readme = (Path(__file__).resolve().parents[2] / "README.md").read_text(encoding="utf-8")
    text = readme.split("\n\n")[0]

    async def scenario():
        settings = Settings(instance=tmp_path / "instance")
        async with application(settings) as app:
            server = RPCServer(app, dispatch, port=0)
            await server.start()
            try:
                async with grpc.aio.insecure_channel(f"127.0.0.1:{server.port}") as channel:
                    namespaces = namespace_pb2_grpc.NamespaceServiceStub(channel)
                    memories = memory_pb2_grpc.MemoryServiceStub(channel)
                    keys = auth_pb2_grpc.AuthServiceStub(channel)
                    jobs = job_pb2_grpc.JobServiceStub(channel)
                    backups = backup_pb2_grpc.BackupServiceStub(channel)
                    admin = (
                        ("authorization", "Bearer " + settings.token_file.read_text().strip()),
                    )
                    with pytest.raises(grpc.aio.AioRpcError) as denied:
                        await namespaces.GetNamespace(
                            namespace_pb2.GetNamespaceRequest(namespace_id="rpc")
                        )
                    assert denied.value.code() == grpc.StatusCode.UNAUTHENTICATED
                    ns = await namespaces.CreateNamespace(
                        namespace_pb2.CreateNamespaceRequest(
                            namespace_id="rpc",
                            config=namespace_pb2.NamespaceConfig(
                                features=namespace_pb2.NamespaceFeatureConfig(
                                    allow_metadata_only_write=True
                                ),
                                relation_extraction=namespace_pb2.RelationExtractionConfig(
                                    default_enabled=False
                                ),
                            ),
                        ),
                        metadata=admin,
                    )
                    assert ns.namespace.namespace_id == "rpc"
                    issued = await keys.CreateApiKey(
                        auth_pb2.CreateApiKeyRequest(namespace_id="rpc", label="Protocol client"),
                        metadata=admin,
                    )
                    key = (("authorization", "Bearer " + issued.secret),)
                    context = common_pb2.RequestContext(
                        namespace_id="rpc",
                        agent_id="caller-cannot-choose-owner",
                        conversation_id="readme",
                    )
                    stored = await memories.IngestText(
                        memory_pb2.IngestTextRequest(
                            context=context,
                            external_id="README.md",
                            source_type="documentation",
                            text=text,
                            relation_extraction=memory_pb2.RELATION_EXTRACTION_MODE_DISABLED,
                        ),
                        metadata=key,
                    )
                    assert stored.stored_ids
                    record = await memories.GetMemory(
                        memory_pb2.GetMemoryRequest(
                            context=context, memory_id=stored.stored_ids[0]
                        ),
                        metadata=key,
                    )
                    assert (
                        record.memory.content == text and record.memory.external_id == "README.md"
                    )
                    searched = await memories.SearchMemory(
                        memory_pb2.SearchMemoryRequest(context=context, query="Nous"), metadata=key
                    )
                    assert any(hit.memory_id == record.memory.memory_id for hit in searched.hits)
                    assert "sparse" in searched.available_channels and searched.retrieval_ready
                    health = await namespaces.GetNamespaceHealth(
                        namespace_pb2.GetNamespaceHealthRequest(namespace_id="rpc"), metadata=key
                    )
                    assert (
                        health.health.namespace.namespace_id == "rpc"
                        and not health.health.embedding.available
                    )
                    wrong = common_pb2.RequestContext(namespace_id="another")
                    with pytest.raises(grpc.aio.AioRpcError) as denied:
                        await memories.GetMemory(
                            memory_pb2.GetMemoryRequest(
                                context=wrong, memory_id=stored.stored_ids[0]
                            ),
                            metadata=key,
                        )
                    assert denied.value.code() == grpc.StatusCode.PERMISSION_DENIED
                    backup = await backups.CreateNamespaceBackup(
                        backup_pb2.CreateNamespaceBackupRequest(namespace_id="rpc"), metadata=admin
                    )
                    chunk = await backups.DownloadNamespaceBackup(
                        backup_pb2.DownloadNamespaceBackupRequest(
                            backup_id=backup.backup.backup_id, max_bytes=65536
                        ),
                        metadata=admin,
                    )
                    assert chunk.data.startswith(b"PK")
                    restored = await backups.RestoreNamespaceFromBackup(
                        backup_pb2.RestoreNamespaceFromBackupRequest(
                            backup_id=backup.backup.backup_id, target_namespace_id="rpc-copy"
                        ),
                        metadata=admin,
                    )
                    assert restored.namespace.namespace_id == "rpc-copy"
                    submitted = await jobs.SubmitDeleteBySource(
                        job_pb2.SubmitDeleteBySourceRequest(
                            context=context, source="documentation:readme"
                        ),
                        metadata=key,
                    )
                    async with asyncio.timeout(15):
                        while True:
                            state = await jobs.GetJob(
                                job_pb2.GetJobRequest(
                                    namespace_id="rpc", job_id=submitted.job.job_id
                                ),
                                metadata=admin,
                            )
                            if state.job.status in {
                                job_pb2.JOB_STATUS_SUCCEEDED,
                                job_pb2.JOB_STATUS_FAILED,
                            }:
                                break
                            await asyncio.sleep(0.05)
                    assert state.job.status == job_pb2.JOB_STATUS_SUCCEEDED, state
                    assert state.job.result["deleted_count"] == 1
                    with pytest.raises(grpc.aio.AioRpcError) as gone:
                        await memories.GetMemory(
                            memory_pb2.GetMemoryRequest(
                                context=context, memory_id=stored.stored_ids[0]
                            ),
                            metadata=key,
                        )
                    assert gone.value.code() == grpc.StatusCode.NOT_FOUND
            finally:
                await server.close()

    asyncio.run(scenario())
