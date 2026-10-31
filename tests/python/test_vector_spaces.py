"""Native space management preserves unknown outcomes and Namespace authorization."""

import asyncio
from pathlib import Path

import httpx
import pytest
from nous_sdk import NousClient, NousError
from nous_sdk.amemorix import AMemorixAdapter
from nous_sdk.contracts import AdmitFiles, CallContext, ImportDocument
from nous_sdk.contracts.credentials import CreateCredential
from nous_sdk.contracts.model_routing import ModelCallOutcome
from nous_sdk.contracts.vector_spaces import CheckVectorSpaces, DeleteVectorSpace, RebuildVectors

from nous.platform.configuration import EmbeddingRoute, Settings
from nous.transports.http import create_app


def test_runtime_and_rebuild_do_not_reissue_unknown_embedding(tmp_path):
    async def scenario():
        settings = Settings(instance=tmp_path / "instance", embedding=EmbeddingRoute(model="retained", dimensions=2))
        app = create_app(settings)
        async with app.router.lifespan_context(app):
            client = NousClient("http://test", credential_file=settings.token_file)
            await client.transport.http.aclose()
            client.transport.http = httpx.AsyncClient(transport=httpx.ASGITransport(app=app), base_url="http://test")
            try:
                namespace = await client.create_namespace("Project")
                subject = await client.create_subject(namespace.id, name="Project")
                text = (await asyncio.to_thread(Path("README.md").read_text, encoding="utf-8"))[:200]
                imported = await client.admit_documents(namespace.id, subject.id, AdmitFiles(
                    inputs=(ImportDocument(text=text, filename="README.md"),), llm_enabled=False, vectors="disabled"))
                service = app.state.application
                ctx = CallContext(caller_id="instance-admin", namespace_id=namespace.id, subject_id=subject.id, credential_source_ids=None)
                record = await service.cognition.get(ctx, imported.outputs[0].cognition[0])
                profile = await service.models.prepare_embedding(ctx)
                request = {"model": profile.route.model, "base_url": profile.route.base_url,
                    "dimensions": profile.route.dimensions, "normalization": profile.route.normalization,
                    "extra_body": profile.route.extra_body, "source_ids": [imported.catalog.units[0].material_id],
                    "texts": [record.body.text]}
                call, _ = await service.models.ledger.reserve(ctx, "embedding", request, 10, invocation_id="existing-unknown")
                await service.models.ledger.finish(ctx, call, None, {}, "ReadTimeout", outcome=ModelCallOutcome(kind="unknown"))
                blocked = await client.rebuild_vectors(namespace.id, subject.id, RebuildVectors(mode="all"))
                assert blocked.projection is None and blocked.blocked_references == (record.reference.exact(),)
                assert len(await service.models.ledger.list(ctx)) == 1
                inventory = await client.vector_spaces(namespace.id, subject.id)
                assert inventory.configured and inventory.configured_space_complete is False
                checked = await client.check_vector_spaces(namespace.id, subject.id, CheckVectorSpaces())
                assert not checked.provider_probed and checked.checked_vectors == 0
                adapter = AMemorixAdapter(client, namespace_id=namespace.id, subject_id=subject.id)
                config = await adapter.memory_runtime_admin(action="get_config")
                assert config["embedding_fingerprint_status"] == "configured" and config["auto_save"] is True
                assert not (await adapter.memory_runtime_admin(action="set_auto_save", enabled=False))["success"]
                assert (await client.persistence_state(namespace.id, subject.id)).auto_save
                scoped = await client.create_credential(CreateCredential(name="Subject vector manager",
                    namespace_ids=(namespace.id,), subject_ids=(subject.id,), capabilities=("read", "write", "manage")))
                limited = NousClient("http://test", token=lambda: scoped.token)
                await limited.transport.http.aclose()
                limited.transport.http = httpx.AsyncClient(transport=httpx.ASGITransport(app=app), base_url="http://test")
                try:
                    await limited.vector_spaces(namespace.id, subject.id)
                    with pytest.raises(NousError) as denied:
                        await limited.delete_vector_space(namespace.id, DeleteVectorSpace(space_id=inventory.spaces[0].space_id))
                    assert denied.value.code == "subject_scope_forbidden"
                finally:
                    await limited.close()
                assert len(await service.models.ledger.list(ctx)) == 1
            finally:
                await client.close()

    asyncio.run(scenario())
