"""Prompt restoration and one role budget across actual request purposes."""

import asyncio
from pathlib import Path

import pytest
from jinja2 import UndefinedError
from nous_sdk.contracts import CallContext

from nous.platform.errors import NousError
from nous.platform.models.ledger import CallLedger
from nous.platform.persistence import Authority
from nous.platform.prompts import PromptService


def test_definitely_not_sent_reservation_resumes_but_reserved_and_unknown_do_not(tmp_path):
    async def scenario():
        from nous_sdk.contracts import DomainError
        from nous_sdk.contracts.model_routing import ModelCallOutcome

        from nous.application import Application
        from nous.platform.configuration import Settings

        app = Application(Settings(instance=tmp_path / "instance"))
        await app.authority.initialize()
        ctx = CallContext(caller_id="owner", namespace_id="n", subject_id="s", credential_source_ids=None)
        try:
            await app.identity.create_namespace(ctx, "N", "namespace")
            await app.identity.create_subject(ctx, "S", "subject", subject_id="s")
            call, _ = await app.models.ledger.reserve(ctx, "formation", {"input": "retained"}, 2, invocation_id="same")
            await app.models.ledger.finish(ctx, call, None, {}, "ConnectError", outcome=ModelCallOutcome(kind="not_sent"))
            recovered, output = await app.models.ledger.reserve(ctx, "formation", {"input": "retained"}, 2, invocation_id="same")
            assert recovered == call and output is None
            with pytest.raises(DomainError) as pending:
                await app.models.ledger.reserve(ctx, "formation", {"input": "retained"}, 2, invocation_id="same")
            assert pending.value.details["status"] == "reserved"
            await app.models.ledger.finish(ctx, call, None, {}, "ReadTimeout", outcome=ModelCallOutcome(kind="unknown"))
            with pytest.raises(DomainError) as unknown:
                await app.models.ledger.reserve(ctx, "formation", {"input": "retained"}, 2, invocation_id="same")
            assert unknown.value.details["error_code"] == "ReadTimeout"
            assert len(await app.models.ledger.list(ctx)) == 1
        finally:
            await app.network.close()
            await app.authority.close()

    asyncio.run(scenario())


def test_accepted_generation_schema_and_prompt_survive_contract_evolution_before_dispatch(tmp_path):
    async def scenario():
        from dataclasses import replace

        from nous_sdk.contracts import DomainError
        from nous_sdk.contracts.growth import GrowthProposal

        from nous.application import Application
        from nous.platform.configuration import Settings

        class ExtendedProposal(GrowthProposal):
            optional_note: str | None = None

        app = Application(Settings(instance=tmp_path / "instance"))
        await app.authority.initialize()
        ctx = CallContext(caller_id="owner", namespace_id="n", subject_id="s", credential_source_ids=None)
        variables = {"maintenance_run_id": "r", "instruction": "", "snapshot": {}, "body_schema": {}}
        try:
            original = await app.models.accept_generation(ctx, "maintenance", variables, invocation_id="frozen")
            key = ("maintenance", "proposal")
            app.models.roles.definitions[key] = replace(app.models.roles.definitions[key], output_type=ExtendedProposal)
            await app.prompts.save(ctx, "maintenance", "New prompt {{ other_input }}", "change-prompt")
            recovered = await app.models.accept_generation(ctx, "maintenance", variables, invocation_id="frozen")
            assert recovered == original
            with pytest.raises(DomainError) as changed:
                await app.models.accept_generation(ctx, "maintenance", {**variables, "instruction": "changed input"}, invocation_id="frozen")
            assert changed.value.code == "model_invocation_conflict"
            assert await app.models.ledger.list(ctx) == []
        finally:
            await app.network.close()
            await app.authority.close()

    asyncio.run(scenario())


def test_prompt_preview_history_restore_and_restart(tmp_path: Path):
    async def scenario():
        path = tmp_path / "authority.sqlite"
        authority = Authority(path)
        prompts = PromptService(authority)
        await authority.initialize()
        ctx = CallContext(
            credential_source_ids=None, caller_id="instance-admin", namespace_id="instance"
        )
        try:
            default = await prompts.get(ctx, "formation")
            assert "source_text" in default["variable_names"]
            await prompts.save(ctx, "formation", "Remember {{ source_text }}", "edit-one")
            preview = prompts.preview(
                {
                    "id": "formation",
                    "revision": None,
                    "template": "Draft {{ text }}",
                    "active": False,
                },
                {"text": "真实材料"},
            )
            assert preview["rendered"] == "Draft 真实材料"
            with pytest.raises(UndefinedError):
                prompts.preview(
                    {
                        "id": "formation",
                        "revision": None,
                        "template": "{{ missing }}",
                        "active": False,
                    },
                    {},
                )
            assert (await prompts.get(ctx, "formation"))["revision"] == 1
            await prompts.save(ctx, "formation", "Changed {{ source_text }}", "edit-two")
            await prompts.activate(ctx, "formation", 1, "restore-one")
            result = await prompts.render(ctx, {"source_text": "原件"}, "formation")
            assert result["rendered"] == "Remember 原件"
            history = await prompts.history(ctx, "formation")
            assert [row["revision"] for row in history] == [2, 1, 0]
            assert [row["revision"] for row in history if row["active"]] == [1]
            await prompts.activate(ctx, "formation", 0, "restore-default")
            assert (await prompts.get(ctx, "formation"))["template_hash"] == default[
                "template_hash"
            ]
        finally:
            await authority.close()
        reopened = Authority(path)
        await reopened.initialize()
        try:
            value = await PromptService(reopened).get(ctx, "formation")
            assert value["revision"] == 0 and value["source"] == "bundled"
        finally:
            await reopened.close()

    asyncio.run(scenario())


def test_embedding_plan_recovers_the_persisted_profile_field_set(tmp_path):
    async def scenario():
        from nous_sdk.contracts import DomainError

        from nous.application import Application
        from nous.platform.configuration import EmbeddingRoute, Settings
        from nous.platform.models.embedding_profile import EmbeddingProfile
        from nous.platform.models.routing import embedding_plan, permissions

        app = Application(Settings(instance=tmp_path / "instance"))
        await app.authority.initialize()
        ctx = CallContext(
            caller_id="owner", namespace_id="ns", subject_id="subject", credential_source_ids=None
        )
        raw = EmbeddingProfile(route=EmbeddingRoute(model="retained")).model_dump(mode="json")
        for key in ("rate_limit_retries", "rate_limit_wait_seconds", "max_retry_after_seconds"):
            raw["fallback_policy"].pop(key)
        inputs = {"texts": ("original",), "media": None, "source_ids": ()}
        try:
            async with app.authority.write(
                ctx,
                "original-intent",
                operation_kind="model.embedding_plan",
                scope="embedding",
                payload={"input": inputs, "permissions": permissions(ctx), "profile": raw},
            ) as tx:
                tx.result(raw)
            profile = EmbeddingProfile.model_validate(raw)
            recovered = await embedding_plan(app.models, ctx, "original-intent", inputs, profile)
            assert recovered.route.model == "retained"
            with pytest.raises(DomainError) as changed:
                await embedding_plan(
                    app.models, ctx, "original-intent", {**inputs, "texts": ("changed",)}, profile
                )
            assert changed.value.code == "model_invocation_conflict"
            assert await app.models.ledger.list(ctx) == []
        finally:
            await app.close()

    asyncio.run(scenario())


def test_text_and_media_share_one_formation_role_budget(tmp_path: Path):
    async def scenario():
        authority = Authority(tmp_path / "authority.sqlite")
        ledger = CallLedger(authority)
        await authority.initialize()
        ctx = CallContext(
            credential_source_ids=None,
            caller_id="worker",
            namespace_id="research",
            subject_id="owner",
        )
        try:
            from nous.identity.service import IdentityService

            identities = IdentityService(authority)
            await identities.create_namespace(ctx, "Research", "namespace")
            await identities.create_subject(ctx, "Owner", "subject", subject_id="owner")
            call_id, _ = await ledger.reserve(
                ctx, "media.formation", {"input": "image"}, 1, budget_role="formation"
            )
            await ledger.finish(ctx, call_id, {"result": "retained"}, {})
            reused, output = await ledger.reserve(
                ctx, "media.formation", {"input": "image"}, 1, budget_role="formation"
            )
            assert reused == call_id and output == {"result": "retained"}
            with pytest.raises(NousError) as caught:
                await ledger.reserve(ctx, "formation", {"input": "text"}, 1)
            assert caught.value.code == "ResourceExhausted"
            # Description has its own configured role allowance.
            await ledger.reserve(
                ctx, "media.description", {"input": "image"}, 1, budget_role="description"
            )
        finally:
            await authority.close()

    asyncio.run(scenario())


def test_http_backup_restores_a_new_instance_only(tmp_path: Path):
    async def scenario():
        import httpx

        from nous.platform.configuration import Settings
        from nous.transports.http import create_app

        settings = Settings(instance=tmp_path / "original")
        app = create_app(settings)
        async with app.router.lifespan_context(app):
            async with httpx.AsyncClient(
                transport=httpx.ASGITransport(app=app),
                base_url="http://test",
                headers={"Authorization": "Bearer " + settings.token_file.read_text().strip()},
            ) as client:
                created = await client.post("/api/backups")
                assert created.status_code == 200
                archive = await client.get("/api/backups/" + created.json()["filename"])
                assert archive.status_code == 200
                restored = await client.post(
                    "/api/backups/restore",
                    params={"instance_name": "restored"},
                    content=archive.content,
                    headers={"Content-Type": "application/zip"},
                )
                assert restored.status_code == 200
                value = restored.json()
                target = tmp_path / "restored"
                assert value["instance"] == str(target)
                assert (target / "data" / "authority.sqlite").is_file()
                new_token = await asyncio.to_thread(Path(value["credential_file"]).read_text)
                original_token = await asyncio.to_thread(settings.token_file.read_text)
                assert new_token != original_token
                again = await client.post(
                    "/api/backups/restore",
                    params={"instance_name": "restored"},
                    content=archive.content,
                )
                assert again.status_code == 409
                escaped = await client.post(
                    "/api/backups/restore",
                    params={"instance_name": "../escape"},
                    content=archive.content,
                )
                assert escaped.status_code == 400
                invalid = await client.post(
                    "/api/backups/restore",
                    params={"instance_name": "invalid"},
                    content=b"broken archive",
                )
                assert invalid.status_code == 400 and not (tmp_path / "invalid").exists()

    asyncio.run(scenario())


def test_durable_model_invocation_keeps_paid_result_across_configuration_changes(tmp_path: Path):
    async def scenario():
        from nous_sdk.contracts import DomainError
        from nous_sdk.contracts.growth import GrowthProposal
        from sqlalchemy import func, select

        from nous.application import Application
        from nous.platform.configuration import Settings
        from nous.platform.models.generation import prepare_invocation
        from nous.platform.models.ledger import calls
        from nous.platform.models.routing import generation_plan

        app = Application(Settings(instance=tmp_path / "instance"))
        await app.authority.initialize()
        ctx = CallContext(
            credential_source_ids=None,
            caller_id="worker",
            namespace_id="research",
            subject_id="owner",
        )
        try:
            await app.identity.create_namespace(ctx, "Research", "namespace")
            await app.identity.create_subject(ctx, "Owner", "subject", subject_id="owner")
            variables = {
                "maintenance_run_id": "one",
                "instruction": "Review retained material",
                "snapshot": {},
                "body_schema": {},
            }
            plan = await generation_plan(
                app.models,
                ctx,
                "maintenance",
                variables,
                GrowthProposal,
                (),
                (),
                "run:one",
                "shared",
                None,
            )
            assert plan is not None
            request = prepare_invocation(
                "maintenance",
                plan.routes[0].route,
                plan.prompt,
                GrowthProposal,
                (),
                (),
            )
            call, output = await app.models.ledger.reserve(
                ctx,
                "maintenance",
                request,
                10,
                invocation_id="run:one",
            )
            assert output is None
            with pytest.raises(DomainError) as pending:
                await app.models.ledger.reserve(
                    ctx,
                    "maintenance",
                    {"prompt": "changed request"},
                    10,
                    invocation_id="run:one",
                )
            assert pending.value.code == "model_invocation_conflict"
            await app.models.ledger.finish(
                ctx,
                call,
                {"summary": "retained result", "rationale": "retained basis", "actions": []},
                {},
            )
            await app.prompts.save(ctx, "maintenance", "Changed prompt {{ source }}", "change")
            # The role has no configured model or key: recovery reads the existing invocation first.
            recovered = await app.models.generate(
                ctx,
                "maintenance",
                variables,
                GrowthProposal,
                invocation_id="run:one",
            )
            assert recovered.call_id == call and recovered.output.summary == "retained result"
            async with app.authority.read(ctx) as view:
                assert await view.connection.scalar(select(func.count()).select_from(calls)) == 1
            unknown, _ = await app.models.ledger.reserve(
                ctx,
                "maintenance",
                {"prompt": "unknown outcome"},
                10,
                invocation_id="run:unknown",
            )
            await app.models.ledger.finish(ctx, unknown, None, {}, "CancelledUnknownOutcome")
            with pytest.raises(DomainError) as unavailable:
                await app.models.generate(
                    ctx, "maintenance", {}, GrowthProposal, invocation_id="run:unknown"
                )
            assert unavailable.value.details["error_code"] == "CancelledUnknownOutcome"
            async with app.authority.write(
                ctx, "erase-before-reserve", operation_kind="source.erase", payload={}
            ) as tx:
                await app.models.ledger.purge_invocations_in(
                    tx, ctx, "maintenance", ("run:erased-before-admission",)
                )
                tx.result(None)
            with pytest.raises(DomainError) as retired:
                await app.models.ledger.reserve(
                    ctx,
                    "maintenance",
                    {"prompt": "already erased"},
                    10,
                    invocation_id="run:erased-before-admission",
                )
            assert retired.value.details["status"] == "purged"
        finally:
            await app.network.close()
            await app.authority.close()

    asyncio.run(scenario())
