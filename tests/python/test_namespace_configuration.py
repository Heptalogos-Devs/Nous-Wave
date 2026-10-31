"""Native namespace policies control registered resources and actual admission."""

import asyncio
from pathlib import Path

import pytest
import sqlalchemy as sa
from nous_sdk.contracts import (
    AdoptCognition,
    CallContext,
    DomainError,
    EvidenceLink,
    ImportDocument,
    MemoryBody,
    QueryRequest,
)
from nous_sdk.contracts.person_profiles import PersonProfilesQuery

from nous.application import Application
from nous.cognition.schema import vectors
from nous.identity.schema import namespaces, subjects
from nous.integrations.amemorix.namespace_config import (
    create_namespace_configured,
    read_namespace_config,
    update_namespace_config,
)
from nous.material.service import materials
from nous.platform.configuration import Settings
from nous.platform.configuration.service import history
from nous.platform.errors import NousError


def test_atomic_configuration_and_quota_preserve_system_facts(tmp_path):
    text = Path("README.md").read_text(encoding="utf-8")

    async def scenario():
        app = Application(Settings(instance=tmp_path))
        await app.authority.initialize()
        ctx = CallContext(caller_id="admin", namespace_id="tenant", credential_source_ids=None)
        try:
            with pytest.raises(NousError, match="quota"):
                await create_namespace_configured(
                    app, ctx, "tenant", {}, {"max_storage_bytes": 1}, "too-small"
                )
            async with app.authority.read(ctx) as view:
                assert not await view.connection.scalar(
                    sa.select(namespaces.c.id).where(namespaces.c.id == "tenant")
                )
                assert not await view.connection.scalar(
                    sa.select(subjects.c.id).where(subjects.c.namespace_id == "tenant")
                )
            namespace = await create_namespace_configured(app, ctx, "tenant", {}, None, "create")
            assert namespace.memory_subject_id is not None
            ctx = ctx.model_copy(update={"subject_id": namespace.memory_subject_id})
            before = await read_namespace_config(app, ctx)
            with pytest.raises(NousError) as missing:
                await update_namespace_config(
                    app,
                    ctx.model_copy(update={"subject_id": None}),
                    {"llm": {"provider_id": "not-registered"}},
                    "unknown",
                )
            assert missing.value.code == "provider_not_found"
            initial = before["config"]
            assert isinstance(initial, dict) and isinstance(initial["llm"], dict)
            reference = initial["llm"]
            with pytest.raises(NousError) as secret:
                await update_namespace_config(
                    app,
                    ctx.model_copy(update={"subject_id": None}),
                    {"llm": {**reference, "secret_ref": "external-payload-secret"}},
                    "secret",
                )
            assert secret.value.code == "provider_secret_reference_invalid"
            updated = await update_namespace_config(
                app,
                ctx.model_copy(update={"subject_id": None}),
                {
                    "llm": reference,
                    "features": {"sparse_retrieval": False},
                    "relation_extraction": {"default_enabled": False},
                },
                "configure",
                expected_config_version=int(before["config_version"]),
                quota={"max_concurrent_requests": 1},
            )
            assert updated == await update_namespace_config(
                app,
                ctx.model_copy(update={"subject_id": None}),
                {
                    "llm": reference,
                    "features": {"sparse_retrieval": False},
                    "relation_extraction": {"default_enabled": False},
                },
                "configure",
                expected_config_version=int(before["config_version"]),
                quota={"max_concurrent_requests": 1},
            )
            async with app.authority.read(ctx) as view:
                changed = (
                    (
                        await view.connection.execute(
                            sa.select(history.c.key).where(
                                history.c.commit_seq == updated["config_version"]
                            )
                        )
                    )
                    .scalars()
                    .all()
                )
                assert set(changed) == {
                    "models",
                    "namespace.features",
                    "formation",
                    "namespace.quota",
                }
            with pytest.raises(NousError) as conflict:
                await update_namespace_config(
                    app,
                    ctx.model_copy(update={"subject_id": None}),
                    {"features": {"sparse_retrieval": True}},
                    "stale",
                    expected_config_version=int(before["config_version"]),
                )
            assert conflict.value.code == "namespace_config_conflict"
            async with app.namespace_limits.request(ctx):
                with pytest.raises(NousError) as busy:
                    async with app.namespace_limits.request(ctx):
                        pass
                assert busy.value.code == "namespace_request_limit"
            async with app.namespace_limits.request(ctx):
                pass
            # Limit user storage while leaving an already accepted system fact intact.
            await update_namespace_config(
                app,
                ctx.model_copy(update={"subject_id": None}),
                {},
                "storage",
                quota={"max_storage_bytes": 1},
            )
            async with app.authority.write(
                ctx,
                "system-fact",
                operation_kind="model.test_receipt",
                payload={"actual": "retained"},
            ) as tx:
                tx.result({"actual": "retained"})
            with pytest.raises(NousError) as full:
                async with app.authority.write(
                    ctx,
                    "new-source",
                    operation_kind="material.import",
                    payload={"filename": "README.md"},
                ) as tx:
                    await app.material.put_in(
                        tx, ctx, ImportDocument(text=text, filename="README.md")
                    )
                    tx.result({"saved": True})
            assert full.value.code == "namespace_storage_limit"
            assert (
                await app.authority.receipt(
                    ctx,
                    "system-fact",
                    operation_kind="model.test_receipt",
                    payload={"actual": "retained"},
                )
                is not None
            )
            async with app.authority.read(ctx) as view:
                assert not await view.connection.scalar(
                    sa.select(materials.c.id).where(materials.c.namespace_id == "tenant")
                )
        finally:
            await app.authority.close()

    asyncio.run(scenario())


def test_features_and_configured_space_use_native_consumers(tmp_path):
    text = Path("README.md").read_text(encoding="utf-8")

    async def scenario():
        app = Application(Settings(instance=tmp_path))
        await app.start()
        ctx = CallContext(caller_id="admin", namespace_id="tenant", credential_source_ids=None)
        try:
            namespace = await create_namespace_configured(
                app,
                ctx,
                "tenant",
                {
                    "features": {
                        "sparse_retrieval": False,
                        "person_profiles": False,
                        "relation_vectors": False,
                        "episodes": False,
                    }
                },
                None,
                "create",
            )
            ctx = ctx.model_copy(update={"subject_id": namespace.memory_subject_id})
            async with app.authority.write(
                ctx, "source", operation_kind="material.import", payload={}
            ) as tx:
                source = await app.material.admit_text_in(
                    tx, ctx, ImportDocument(text=text, filename="README.md")
                )
                memory = await app.cognition.adopt_in(
                    tx,
                    ctx,
                    AdoptCognition(
                        body=MemoryBody(text=text),
                        evidence=(EvidenceLink(observation_id=source.event["id"]),),
                    ),
                )
                tx.result(memory)
            with pytest.raises(DomainError) as sparse:
                await app.retrieval.query(ctx, QueryRequest(text="Nous", mode="lexical"))
            assert sparse.value.code == "namespace_feature_disabled"
            exact = await app.retrieval.query(
                ctx, QueryRequest(reference=memory.reference.exact(), mode="lexical")
            )
            assert [hit.object.reference for hit in exact.hits] == [memory.reference]
            with pytest.raises(DomainError) as profiles:
                await app.person_profiles.query(ctx, PersonProfilesQuery())
            assert profiles.value.code == "namespace_feature_disabled"
            # Metadata configuration selects a real registered space; no provider request is made.
            resources = (await app.configuration.effective(ctx, "models"))["value"]
            assert isinstance(resources, dict)
            resources["providers"]["embedding"]["base_url"] = "http://127.0.0.1:1/v1"
            resources["models"]["embedding"]["name"] = "registered-embedding"
            await app.configuration.set(ctx, "models", resources, "resources", "namespace")
            from nous.platform.models.embedding import vector_space

            profile = await app.models.prepare_embedding(ctx)
            route = profile.route
            async with app.authority.write(
                ctx, "old-space", operation_kind="vectors.metadata", payload={}
            ) as tx:
                await tx.connection.execute(
                    vectors.insert().values(
                        reference=memory.reference.exact(),
                        space="old-space",
                        model=route.model,
                        dimensions=route.dimensions,
                        vector=[0] * route.dimensions,
                        commit_seq=tx.commit_seq,
                    )
                )
                tx.result({"saved": True})
            status = await app.retrieval.status(ctx)
            assert status.spaces and status.configured_space_complete is False
            from nous.retrieval.media import media_records_in

            async with app.authority.read(ctx) as view:
                target_references = [
                    memory.reference.exact(),
                    *(
                        item.reference.exact()
                        for item in await media_records_in(
                            view, ctx, app.material, app.derivations, include_text=False
                        )
                    ),
                ]
            async with app.authority.write(
                ctx, "current-space", operation_kind="vectors.metadata", payload={}
            ) as tx:
                await tx.connection.execute(
                    vectors.insert(),
                    [
                        dict(
                            reference=reference,
                            space=vector_space(route, route.model),
                            model=route.model,
                            dimensions=route.dimensions,
                            vector=[0] * route.dimensions,
                            commit_seq=tx.commit_seq,
                        )
                        for reference in target_references
                    ],
                )
                tx.result({"saved": True})
            assert (await app.retrieval.status(ctx)).configured_space_complete is True
        finally:
            await app.close()

    asyncio.run(scenario())
