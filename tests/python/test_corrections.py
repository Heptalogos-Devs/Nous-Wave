"""Corrections preserve later knowledge and one atomic, source-backed adoption."""

import asyncio

import httpx
from nous_sdk import NousClient
from nous_sdk.amemorix import AMemorixAdapter
from nous_sdk.contracts import AdmitFiles, ImportDocument, MemoryBody
from nous_sdk.contracts.corrections import (
    ApplyCorrection,
    CorrectionEdit,
    PlanCorrection,
    RollbackCorrection,
)
from nous_sdk.contracts.facts import ChatFactScopeInput, FactChange, FactClaimInput
from nous_sdk.contracts.records import RecordsQuery

from nous.platform.configuration import Settings
from nous.transports.http import create_app


def test_saved_corrections_are_atomic_and_rollback_respects_later_revisions(tmp_path):
    async def scenario():
        settings = Settings(instance=tmp_path / "instance")
        app = create_app(settings)
        async with app.router.lifespan_context(app):
            async with NousClient("http://test", credential_file=settings.token_file) as api:
                await api.transport.http.aclose()
                api.transport.http = httpx.AsyncClient(
                    transport=httpx.ASGITransport(app=app), base_url="http://test"
                )
                namespace = await api.create_namespace("Correction sources")
                subject = await api.create_subject(namespace.id)
                imported = await api.admit_documents(
                    namespace.id,
                    subject.id,
                    AdmitFiles(
                        inputs=(
                            ImportDocument(text="会议在周二。"),
                            ImportDocument(text="地点在一楼。"),
                        ),
                        llm_enabled=False,
                        vectors="disabled",
                    ),
                )
                records = tuple(
                    [
                        (
                            await api.record_detail(namespace.id, subject.id, output.cognition[0])
                        ).entry.record
                        for output in imported.outputs
                    ]
                )
                assert all(isinstance(record.body, MemoryBody) for record in records)
                edits = tuple(
                    CorrectionEdit(
                        reference=record.reference,
                        body=record.body.model_copy(update={"text": text}),
                    )
                    for record, text in zip(records, ("会议在周四。", "地点在二楼。"), strict=True)
                )
                plan = await api.plan_correction(
                    namespace.id, subject.id, PlanCorrection(edits=edits)
                )
                adopted = await api.apply_correction(
                    namespace.id,
                    subject.id,
                    ApplyCorrection(operation_id="adopt-once", plan_id=plan.id),
                )
                assert len(adopted.records) == 2 and not adopted.conflicts
                assert (
                    await api.apply_correction(
                        namespace.id,
                        subject.id,
                        ApplyCorrection(operation_id="adopt-once", plan_id=plan.id),
                    )
                    == adopted
                )
                later_record = adopted.records[1]
                later = await api.plan_correction(
                    namespace.id,
                    subject.id,
                    PlanCorrection(
                        edits=(
                            CorrectionEdit(
                                reference=later_record.reference,
                                body=later_record.body.model_copy(
                                    update={"text": "地点改为三楼。"}
                                ),
                            ),
                        )
                    ),
                )
                later_result = await api.apply_correction(
                    namespace.id, subject.id, ApplyCorrection(plan_id=later.id)
                )
                undone = await api.rollback_correction(
                    namespace.id, subject.id, RollbackCorrection(plan_id=plan.id)
                )
                assert len(undone.records) == 1 and len(undone.conflicts) == 1
                assert undone.records[0].body.text == "会议在周二。"
                assert (
                    await api.cognitive_object(later_record.reference.id)
                ).body.text == "地点改为三楼。"
                stale = await api.plan_correction(
                    namespace.id,
                    subject.id,
                    PlanCorrection(
                        edits=(
                            edits[0],
                            CorrectionEdit(
                                reference=later_result.records[0].reference,
                                body=later_result.records[0].body.model_copy(
                                    update={"text": "不应生效。"}
                                ),
                            ),
                        ),
                    ),
                )
                rejected = await api.apply_correction(
                    namespace.id, subject.id, ApplyCorrection(plan_id=stale.id)
                )
                assert rejected.conflicts and not rejected.records
                assert (
                    await api.cognitive_object(later_record.reference.id)
                ).body.text == "地点改为三楼。"

                fact = await api.change_fact(
                    namespace.id,
                    subject.id,
                    FactChange(
                        action="assert",
                        claim=FactClaimInput(
                            scope=ChatFactScopeInput(conversation_ref="room-one"),
                            fact_key="meeting",
                            value_text="Tuesday",
                        ),
                    ),
                )
                fact_plan = await api.plan_correction(
                    namespace.id,
                    subject.id,
                    PlanCorrection(
                        edits=(
                            CorrectionEdit(
                                reference=fact.record.reference,
                                body=fact.record.body.model_copy(update={"value_text": "Thursday"}),
                            ),
                        )
                    ),
                )
                fact_result = await api.apply_correction(
                    namespace.id, subject.id, ApplyCorrection(plan_id=fact_plan.id)
                )
                assert len(fact_result.records) == 2
                restored = await api.rollback_correction(
                    namespace.id, subject.id, RollbackCorrection(plan_id=fact_plan.id)
                )
                assert len(restored.records) == 2 and not restored.conflicts
                current_facts = await api.records(
                    namespace.id, subject.id, RecordsQuery(kinds=("fact_claim",))
                )
                assert (
                    len(current_facts.items) == 1
                    and current_facts.items[0].record.body.value_text == "Tuesday"
                )

                adapter = AMemorixAdapter(api, namespace_id=namespace.id, subject_id=subject.id)
                assert (await adapter.memory_correction_admin(action="get", plan_id=plan.id))[
                    "plan"
                ]["id"] == plan.id
                fetched = await adapter.memory_fuzzy_modify_admin(action="get", plan_id=plan.id)
                assert fetched["plan"]["results"][-1]["action"] == "rollback"
                await adapter.maintain_memory(
                    action="purge", target=records[0].reference.id, force=True
                )
                redacted = await api.correction_plan(namespace.id, subject.id, plan.id)
                assert (
                    redacted.state == "unavailable"
                    and not redacted.targets
                    and not redacted.changes
                    and not redacted.results
                )

    asyncio.run(scenario())
