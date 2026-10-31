"""Native source/object lifecycle and legacy selectors share one authorized operation."""

import asyncio
from pathlib import Path

import httpx
import pytest
from nous_sdk import NousClient, NousError
from nous_sdk.amemorix import AMemorixAdapter
from nous_sdk.contracts import (
    AdmitFiles,
    AdoptCognition,
    AssociationBody,
    EntityBody,
    EvidenceLink,
    ImportDocument,
    MemoryChange,
    RecordsQuery,
    SourceCommand,
    SourceResult,
)
from nous_sdk.contracts.management import SourceSelection
from nous_sdk.contracts.retention import RetentionExpectation

from nous.platform.configuration import Settings
from nous.transports.http import create_app


def test_object_modes_and_source_counts_retain_exact_operation_scope(tmp_path):
    text = (
        (Path(__file__).resolve().parents[2] / "README.md")
        .read_text(encoding="utf-8")
        .split("\n\n")[0]
    )

    async def scenario():
        settings = Settings(instance=tmp_path / "instance")
        app = create_app(settings)
        async with app.router.lifespan_context(app):
            api = NousClient("http://test", credential_file=settings.token_file)
            await api.transport.http.aclose()
            api.transport.http = httpx.AsyncClient(
                transport=httpx.ASGITransport(app=app), base_url="http://test"
            )
            try:
                ns = await api.create_namespace("Source operation")
                subject = await api.create_subject(ns.id)
                admitted = await api.admit_documents(
                    ns.id,
                    subject.id,
                    AdmitFiles(
                        inputs=(
                            ImportDocument(
                                text=text, filename="README.md", source="repo:README.md"
                            ),
                        ),
                        llm_enabled=False,
                        vectors="disabled",
                    ),
                )
                memory = admitted.outputs[0].cognition[0]
                evidence = (EvidenceLink(reference=memory),)
                project = await api.adopt_cognition(
                    ns.id,
                    subject.id,
                    AdoptCognition(
                        body=EntityBody(label="Nous", entity_type="other"), evidence=evidence
                    ),
                )
                document = await api.adopt_cognition(
                    ns.id,
                    subject.id,
                    AdoptCognition(
                        body=EntityBody(label="README", entity_type="other"), evidence=evidence
                    ),
                )
                association = await api.adopt_cognition(
                    ns.id,
                    subject.id,
                    AdoptCognition(
                        body=AssociationBody(
                            from_ref=project.reference,
                            to_ref=document.reference,
                            relation="documented_by",
                            basis_class="source_evidence",
                        ),
                        evidence=evidence,
                    ),
                )
                adapter = AMemorixAdapter(api, namespace_id=ns.id, subject_id=subject.id)
                preview = await api.preview_sources(
                    ns.id,
                    subject.id,
                    selection=SourceSelection(
                        records=RecordsQuery(kinds=("entity",), text="Nous", text_match="label")
                    ),
                )
                assert {ref.id for ref in preview.references} == {
                    project.reference.id,
                    association.reference.id,
                }
                changed = await api.sources(
                    {
                        "action": "trash",
                        "request": {
                            "namespace_id": ns.id,
                            "subject_id": subject.id,
                            "references": [project.reference.model_dump(mode="json")],
                            "reason": "Remove the selected project node",
                            "expected_preview": preview.preview_token,
                        },
                    }
                )
                node_removal = SourceResult.model_validate(changed)
                assert len(node_removal.references) == 2
                restored = await adapter.call(
                    "memory_delete_admin",
                    {
                        "action": "restore",
                        "mode": "entity",
                        "operation_id": node_removal.source_operation_id,
                    },
                )
                assert restored["success"]
                relations = await api.records(
                    ns.id, subject.id, RecordsQuery(kinds=("association",))
                )
                relation = relations.items[0].record.reference
                plan = await api.preview_sources(
                    ns.id, subject.id, selection=SourceSelection(references=(relation,))
                )
                state = next(
                    item for item in plan.retention_states if item.reference.id == relation.id
                )
                expectation = RetentionExpectation.model_validate(
                    state.model_dump(exclude={"score"})
                )
                with pytest.raises(NousError) as stale:
                    await api.trash_sources(
                        ns.id,
                        subject.id,
                        SourceCommand(
                            references=(relation,),
                            retention_expectations=(
                                expectation.model_copy(update={"strength": state.strength / 2}),
                            ),
                            reason="Reject changed retention state",
                        ),
                    )
                assert stale.value.code == "retention_precondition_failed"
                expected_state = {
                    "expected_lifecycle_revision": state.reference.revision,
                    "expected_retention_strength": state.strength,
                    "expected_retention_anchor_at": state.anchor_unix_ms / 1000,
                    "expected_inactive_since": None
                    if state.inactive_since_unix_ms is None
                    else state.inactive_since_unix_ms / 1000,
                    "expected_inactive_reason": state.inactive_reason,
                    "expected_is_inactive": state.inactive,
                    "expected_is_permanent": state.permanent,
                    "expected_is_pinned": state.pinned,
                    "expected_protected_until": None
                    if state.protected_until_unix_ms is None
                    else state.protected_until_unix_ms / 1000,
                }
                deleted_relation = await adapter.call(
                    "memory_delete_admin",
                    {
                        "action": "execute",
                        "mode": "relation",
                        "selector": {
                            "hash": relation.id,
                            "expected_relation_states": {relation.id: expected_state},
                        },
                    },
                )
                assert deleted_relation["deleted_relation_count"] == 1
                restored_relation = await adapter.call(
                    "memory_delete_admin",
                    {
                        "action": "restore",
                        "mode": "relation",
                        "selector": {"query": "documented_by"},
                    },
                )
                assert restored_relation["success"]
                trashed = await api.update_memory(
                    ns.id, subject.id, MemoryChange(reference=memory, action="trash")
                )
                source_preview = await api.preview_sources(
                    ns.id, subject.id, selection=SourceSelection(source_names=("repo:README.md",))
                )
                assert memory.id not in {ref.id for ref in source_preview.retiring}
                source_removed = await api.trash_sources(
                    ns.id,
                    subject.id,
                    SourceCommand(
                        source_names=("repo:README.md",),
                        reason="Withdraw the original source",
                        expected_preview=source_preview.preview_token,
                    ),
                )
                assert not any(ref.kind == "memory" for ref in source_removed.retired)
                await api.restore_sources(
                    ns.id,
                    subject.id,
                    SourceCommand(
                        observation_ids=source_removed.observation_ids,
                        reason="Restore the original source",
                    ),
                )
                await api.update_memory(
                    ns.id, subject.id, MemoryChange(reference=trashed.reference, action="restore")
                )
                selector = {
                    "paragraph_hashes": [memory.id],
                    "entity_hashes": [document.reference.id],
                }
                mixed = await adapter.call(
                    "memory_delete_admin",
                    {"action": "execute", "mode": "mixed", "selector": selector},
                )
                assert (
                    mixed["deleted_paragraph_count"]
                    == mixed["deleted_entity_count"]
                    == mixed["deleted_relation_count"]
                    == 1
                )
                assert (
                    await adapter.call(
                        "memory_delete_admin",
                        {
                            "action": "restore",
                            "mode": "mixed",
                            "operation_id": mixed["operation_id"],
                        },
                    )
                )["success"]
                mixed = await adapter.call(
                    "memory_delete_admin",
                    {"action": "execute", "mode": "mixed", "selector": selector},
                )
                sweep = await adapter.call(
                    "memory_delete_admin", {"action": "purge", "grace_hours": 24}
                )
                assert sweep["purged_count"] == 0
                operation = await api.source_operation(
                    ns.id, subject.id, source_operation_id=mixed["operation_id"]
                )
                selected = SourceResult.model_validate(operation["result"]).references
                permanent = await api.preview_sources(
                    ns.id,
                    subject.id,
                    selection=SourceSelection(references=selected),
                    action="purge",
                )
                assert project.reference.id in {ref.id for ref in permanent.affected}
                with pytest.raises(NousError, match="grace"):
                    await api.purge_sources(
                        ns.id,
                        subject.id,
                        SourceCommand(
                            references=selected,
                            grace_hours=24,
                            reason="Respect the recorded deletion period",
                        ),
                    )
                result = await api.purge_sources(
                    ns.id,
                    subject.id,
                    SourceCommand(
                        references=selected,
                        reason="Permanently remove the selected graph and paragraph",
                        expected_preview=permanent.preview_token,
                    ),
                )
                assert result.source_operation_id
                log = await api.source_operation(
                    ns.id, subject.id, source_operation_id=result.source_operation_id
                )
                assert log["kind"] == "purge"
                history = await api.records(
                    ns.id,
                    subject.id,
                    RecordsQuery(object_ids=(memory.id,), view="history", purpose="management"),
                )
                assert history.items and all(entry.record.body is None for entry in history.items)
                listed = await adapter.call(
                    "memory_delete_admin", {"action": "list_operations", "mode": "mixed"}
                )
                assert listed["count"] > 0
            finally:
                await api.close()

    asyncio.run(scenario())
