"""Associations use the same lifecycle owner, visibility and exact Source preconditions."""

import asyncio
from hashlib import sha256
from pathlib import Path

import httpx
import pytest
from nous_sdk import NousClient, NousError
from nous_sdk.amemorix import AMemorixAdapter
from nous_sdk.contracts import (
    AdmitFiles,
    AdoptCognition,
    AssociationBody,
    CallContext,
    EntityBody,
    EvidenceLink,
    ImportDocument,
    MemoryChange,
    QueryRequest,
    RetentionChange,
    RetentionExpectation,
)
from nous_sdk.contracts.context import (
    ChangeResidents,
    CognitiveTarget,
    ContextPrepare,
    ContextResidentAdmission,
    ContextUsage,
    OpenContextSession,
)

from nous.platform.configuration import Settings
from nous.transports.http import create_app


def test_association_lifecycle_is_shared_and_scoped(tmp_path):
    async def scenario():
        settings = Settings(instance=tmp_path / "instance")
        http = create_app(settings)
        async with http.router.lifespan_context(http):
            api = NousClient("http://test", credential_file=settings.token_file)
            await api.transport.http.aclose()
            api.transport.http = httpx.AsyncClient(transport=httpx.ASGITransport(app=http), base_url="http://test")
            try:
                namespace = await api.create_namespace("Project")
                subject = await api.create_subject(namespace.id, name="Project")
                text = (await asyncio.to_thread(Path("README.md").read_text, encoding="utf-8"))[:2000]
                imported = await api.admit_documents(namespace.id, subject.id, AdmitFiles(
                    inputs=(ImportDocument(text=text, filename="README.md"),), llm_enabled=False, vectors="disabled"))
                evidence = (EvidenceLink(reference=imported.outputs[0].cognition[0]),)
                nous = await api.adopt_cognition(namespace.id, subject.id, AdoptCognition(body=EntityBody(label="Nous", entity_type="object"), evidence=evidence))
                wave = await api.adopt_cognition(namespace.id, subject.id, AdoptCognition(body=EntityBody(label="Wave", entity_type="object"), evidence=evidence))
                relation_hash = sha256(b"Nous/Wave").hexdigest()
                relation = await api.adopt_cognition(namespace.id, subject.id, AdoptCognition(id=relation_hash,
                    body=AssociationBody(from_ref=nous.reference, to_ref=wave.reference, relation="检索", basis_class="source_evidence", weight=0.7), evidence=evidence))
                assert relation.lifecycle and relation.lifecycle.strength == 1
                session = await api.open_context_session(namespace.id, subject.id, OpenContextSession(session_id="project"))
                await api.change_context_residents(namespace.id, subject.id, ChangeResidents(session_id=session.id,
                    expected_revision=session.revision, action="admit", residents=(ContextResidentAdmission(
                        target=CognitiveTarget(reference=relation.reference), entry_reason="Project relationship"),)))
                adapter = AMemorixAdapter(api, namespace_id=namespace.id, subject_id=subject.id)
                frozen = await adapter.maintain_memory("freeze", target=relation_hash)
                assert frozen["memory"] is None and frozen["relations"][0]["body"]["kind"] == "association"
                assert not (await api.query_cognition(namespace.id, subject.id, QueryRequest(text="检索", mode="lexical", kinds=("association",)))).hits
                from nous_sdk.contracts.graph_management import GraphReadQuery
                assert not any(record.reference.kind == "association" for record in (await api.read_graph(namespace.id, subject.id, GraphReadQuery())).records)
                prepared = await api.prepare_context(namespace.id, subject.id, ContextPrepare(session_id=session.id,
                    consumer_id="reply", instance_id="host", usage=ContextUsage(purpose="Project")))
                assert not prepared.response.projection.segments
                forever = await adapter.memory_v5_admin(action="remember_forever", target=relation_hash)
                assert forever["records"][0]["lifecycle"]["permanent"]
                status = await adapter.memory_v5_admin(action="status", target=relation_hash)
                item = status["items"][0]
                assert item["is_permanent"] is True and item["lifecycle_revision"] > 1
                assert item["retention_strength"] == 1 and item["is_inactive"] is False
                from nous_sdk.contracts import CognitiveRef, DomainError
                ref = CognitiveRef.model_validate(forever["records"][0]["reference"])
                snapshot = await api.retention_snapshot(namespace.id, subject.id, ref)
                expectation = RetentionExpectation.model_validate(snapshot.model_dump(exclude={"score"}))
                with pytest.raises(NousError) as protected:
                    await api.change_retention(namespace.id, subject.id, RetentionChange(reference=ref, action="trash"))
                assert protected.value.code == "retention_protected"
                changed = await api.change_retention(namespace.id, subject.id, RetentionChange(reference=ref, action="allow_decay"))
                service = http.state.application
                ctx = CallContext(caller_id="instance-admin", namespace_id=namespace.id, subject_id=subject.id, credential_source_ids=None)
                with pytest.raises(DomainError) as stale:
                    async with service.authority.write(ctx, "stale", operation_kind="check", payload={}) as tx:
                        await service.cognition.require_retention_preconditions_in(tx, ctx, (expectation,))
                assert stale.value.code == "retention_precondition_failed"
                assert changed.body.weight == relation.body.weight
                with pytest.raises(NousError) as memory_only:
                    await api.update_memory(namespace.id, subject.id, MemoryChange(reference=changed.reference, action="freeze"))
                assert memory_only.value.code == "memory_required"
                await api.update_memory(namespace.id, subject.id, MemoryChange(reference=imported.outputs[0].cognition[0], action="protect", hours=1))
                assert await service.models.ledger.list(ctx) == []
            finally:
                await api.close()

    asyncio.run(scenario())
