"""Feedback uses actual same-conversation contacts; empty observations have no paid effects."""

import asyncio
from pathlib import Path

import pytest
from nous_sdk.amemorix.contracts import AMemorixOperation
from nous_sdk.contracts import (
    AdmitFiles,
    CallContext,
    ContextCommand,
    ContextFeedback,
    DomainError,
    ImportDocument,
)
from nous_sdk.contracts.common import utc_now
from nous_sdk.contracts.feedback import EnqueueFeedback, FeedbackPolicy, FeedbackQuery

from nous.bootstrap import application
from nous.integrations.amemorix.service import AMemorixService
from nous.platform.configuration import Settings


def test_feedback_disabled_empty_and_real_context_contact(tmp_path):
    async def scenario():
        async with application(Settings(instance=tmp_path / "instance")) as app:
            ctx = CallContext(caller_id="host", namespace_id="n", subject_id="s", credential_source_ids=None)
            await app.identity.create_namespace(ctx, "Project", "namespace")
            await app.identity.create_subject(ctx, "Project", "subject", subject_id="s")
            text = await asyncio.to_thread(Path("README.md").read_text, encoding="utf-8")
            imported = await app.imports.admit_files(ctx, AdmitFiles(
                inputs=(ImportDocument(text=text[:800], filename="README.md"),), llm_enabled=False, vectors="disabled"))
            refs = imported.outputs[0].cognition
            disabled = await app.feedback.enqueue(ctx, EnqueueFeedback(references=refs, session_id="project"))
            assert disabled.state == "disabled" and disabled.job_id is None
            adapter = AMemorixService(app)
            assert (await adapter.execute(ctx, AMemorixOperation(tool="enqueue_feedback_task", arguments={
                "query_tool_id": "q1", "session_id": "project"}))) ["reason"] == "feedback_correction_disabled"
            await app.configuration.set(ctx, "feedback", FeedbackPolicy(enabled=True, window_hours=0.001), "enable", scope="subject")
            empty = await app.feedback.enqueue(ctx, EnqueueFeedback(references=refs, session_id="no-contacts"))
            admission = await app.contexts.admit(ctx, ContextCommand(references=refs, purpose="Read project", session_id="project"))
            queued = await adapter.execute(ctx, AMemorixOperation(tool="enqueue_feedback_task", arguments={
                "query_tool_id": "q2", "session_id": "project", "query_timestamp": utc_now().coordinate / 1000,
                "structured_content": {"hits": [{"metadata": {"reference": refs[0].model_dump(mode="json")}}]}}))
            event = await app.contexts.record_feedback(ctx, ContextFeedback(context_id=admission["id"], reference=refs[0],
                kind="actual_use", description=text[800:1100]))
            await app.execution.wait(ctx, "feedback-contact:" + str(event["id"]))
            assert (await app.feedback.get(ctx, empty.id)).state == "pending"
            await app.execution.wait(ctx, empty.job_id)
            assert (await app.feedback.get(ctx, empty.id)).state == "no_feedback"
            with pytest.raises(DomainError) as unavailable:
                await app.execution.wait(ctx, str(queued["job_id"]))
            assert unavailable.value.code == "model_unavailable"
            frozen = await app.feedback.get(ctx, str(queued["native_task_id"]))
            assert frozen.input.messages and frozen.supports
            support = await app.cognition.get(ctx, frozen.supports[0])
            assert support.body.contact_observation_id and support.body.text == text[800:1100].strip()
            assert await app.models.ledger.list(ctx) == []
            listed = await adapter.execute(ctx, AMemorixOperation(tool="memory_feedback_admin", arguments={"action": "get", "task_id": queued["task_id"]}))
            assert listed["success"]
            assert len((await app.feedback.list(ctx, FeedbackQuery())).items) == 3

    asyncio.run(scenario())
