"""Creator documents use the shared typed proposal and exact revision adoption path."""

from nous_sdk.contracts import (
    CallContext,
    DomainError,
    FreezeCatalog,
)
from nous_sdk.contracts.ego_model import AdoptSeed, InterpretSeed, SeedDocument
from nous_sdk.contracts.maintenance import MaintenanceRun, MaintenanceSubmission, StartMaintenance
from pydantic import JsonValue, TypeAdapter

from nous.cognition.service import CognitionService
from nous.identity.access import require_subject_in
from nous.imports.service import ImportService
from nous.maintenance.service import MaintenanceService
from nous.platform.persistence import Authority


class SeedService:
    def __init__(
        self,
        authority: Authority,
        cognition: CognitionService,
        maintenance: MaintenanceService,
        imports: ImportService,
    ) -> None:
        self.authority, self.cognition, self.maintenance = authority, cognition, maintenance
        self.imports = imports

    async def interpret(self, ctx: CallContext, command: InterpretSeed) -> MaintenanceSubmission:
        async with self.authority.read(ctx) as view:
            await require_subject_in(view, ctx, "ego")
        existing = await self.maintenance.existing_seed(ctx, command.operation_id)
        if existing is not None:
            if (existing.command.instruction, existing.command.previous_seed_id) != (
                command.instruction,
                command.previous_seed_id,
            ):
                raise DomainError(
                    "seed_invocation_conflict",
                    "The existing seed interpretation has different input",
                )
            if existing.command.source_catalog_id is None:
                proof = await self.authority.receipt(
                    ctx,
                    command.operation_id + ":documents",
                    operation_kind="self.seed",
                    scope=ctx.require_subject(),
                    payload=AdoptSeed(
                        operation_id=command.operation_id + ":documents",
                        documents=tuple(
                            SeedDocument(document=document) for document in command.documents
                        ),
                    ),
                )
                if proof is None:
                    raise DomainError(
                        "seed_input_proof_unavailable",
                        "The saved interpretation requires its original source-admission proof",
                    )
            else:
                await self.imports.freeze_catalog(
                    ctx,
                    FreezeCatalog(
                        operation_id=command.operation_id + ":materials", inputs=command.documents
                    ),
                )
            return MaintenanceSubmission(run_id=existing.id, job_id=existing.job_id)
        previous_references = ()
        if command.previous_seed_id is not None:
            previous = await self.maintenance.read(ctx, command.previous_seed_id)
            if (
                previous.command.role != "seed"
                or previous.review is None
                or previous.review.decision != "adopt"
            ):
                raise DomainError(
                    "seed_version_invalid", "Previous seed must be an adopted version"
                )
            previous_references = previous.review.adopted
        frozen = FreezeCatalog(
            operation_id=command.operation_id + ":materials",
            inputs=command.documents,
        )
        catalog = await self.imports.freeze_catalog(ctx, frozen)
        if not catalog.units or catalog.errors:
            raise DomainError(
                "seed_input_invalid", "Creator materials must have supported readable sources"
            )
        effective = ctx
        if ctx.source_groups():
            async with self.authority.read(ctx) as view:
                effective = (
                    await self.cognition.materials.restore_input_in(
                        view,
                        ctx,
                        operation_id=frozen.operation_id,
                        operation_kind="imports.freeze",
                        scope=ctx.require_subject(),
                    )
                ).context
        return await self.maintenance.submit(
            effective,
            StartMaintenance(
                operation_id=command.operation_id,
                instruction=command.instruction,
                references=previous_references,
                source_catalog_id=catalog.id,
                role="seed",
                previous_seed_id=command.previous_seed_id,
                limit=1000,
            ),
        )

    async def import_and_adopt(self, ctx: CallContext, command: InterpretSeed) -> MaintenanceRun:
        submission = await self.interpret(ctx, command)
        current = await self.maintenance.read(ctx, submission.run_id)
        await self.maintenance.execution.wait(ctx, current.job_id)
        run = await self.maintenance.read(ctx, submission.run_id)
        if run.review is None:
            from nous_sdk.contracts.maintenance import ReviewMaintenance

            run = await self.maintenance.review(
                ctx,
                run.id,
                ReviewMaintenance(
                    operation_id=command.operation_id + ":adopt",
                    expected_revision=run.revision,
                    decision="adopt",
                    reason="Creator imported and adopted the interpreted seed",
                ),
            )
        return run

    async def markdown(self, ctx: CallContext) -> str:
        records = await self.cognition.ego_profile(ctx)
        lines = ["# 当前人格", ""]
        for record in records:
            if record.body is None:
                continue
            body = TypeAdapter(dict[str, JsonValue]).validate_python(
                record.body.model_dump(mode="json")
            )
            title = (
                body.get("facet")
                or body.get("domain")
                or body.get("title")
                or record.reference.kind
            )
            lines.extend([f"## {title}", ""])
            for field in ("description", "understanding", "narrative", "uncertainty"):
                value = body.get(field)
                if isinstance(value, str) and value:
                    lines.extend([value, ""])
            for field, label in (("contexts", "适用情境"), ("exceptions", "例外")):
                values = body.get(field)
                if isinstance(values, list) and values:
                    lines.extend([f"{label}：" + "；".join(str(value) for value in values), ""])
            lines.extend([f"修订：{record.reference.exact()}", ""])
            if record.source_roots:
                lines.extend(["来源：" + "、".join(record.source_roots), ""])
        return "\n".join(lines)
