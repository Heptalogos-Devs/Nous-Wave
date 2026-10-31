"""A-Memorix correction and fuzzy-modify names share the native correction owner."""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal
from uuid import uuid4

from nous_sdk.amemorix.contracts import ProfileInput
from nous_sdk.contracts import CallContext, Contract, DomainError
from nous_sdk.contracts.corrections import (
    ApplyCorrection,
    CorrectionEdit,
    CorrectionPlansQuery,
    PlanCorrection,
    RollbackCorrection,
)
from nous_sdk.contracts.records import RecordsQuery
from pydantic import AliasChoices, Field, JsonValue

from .helpers import json_object
from .profile_read import select_person

if TYPE_CHECKING:
    from .service import AMemorixService


class CorrectionAdminInput(Contract):
    action: Literal["plan", "preview", "execute", "get", "list", "rollback"]
    request_text: str = Field(default="", validation_alias=AliasChoices("request_text", "text"))
    scope: str = "person_profile"
    person_id: str = ""
    person_keyword: str = Field(
        default="", validation_alias=AliasChoices("person_keyword", "keyword")
    )
    chat_id: str = ""
    limit: int = Field(default=50, ge=1, le=1000)
    plan_id: str = ""
    confirmed: bool = False
    requested_by: str = ""
    reason: str = ""
    statuses: tuple[str, ...] = ()
    status: str | None = None
    cursor: str | None = None
    operation_id: str | None = None
    edits: tuple[CorrectionEdit, ...] = ()


async def correction_admin(
    self: AMemorixService, ctx: CallContext, arguments: dict[str, JsonValue]
) -> dict[str, JsonValue]:
    command = CorrectionAdminInput.model_validate(arguments)
    if command.action == "get":
        plan = await self.app.corrections.get(ctx, command.plan_id)
        return json_object({"success": True, "plan": plan})
    if command.action == "list":
        page = await self.app.corrections.list(
            ctx, CorrectionPlansQuery(cursor=command.cursor, limit=command.limit)
        )
        statuses = set(command.statuses)
        if command.status:
            statuses.add(command.status)
        items = tuple(plan for plan in page.items if not statuses or plan.state in statuses)
        return json_object(
            {"success": True, "items": items, "count": len(items), "next_cursor": page.next_cursor}
        )
    operation_id = command.operation_id or str(uuid4())
    if command.action == "execute":
        if not command.confirmed:
            return {
                "success": False,
                "error": "Adoption requires confirmed=true for the saved plan",
            }
        result = await self.app.corrections.apply(
            ctx, ApplyCorrection(operation_id=operation_id, plan_id=command.plan_id)
        )
        return json_object(
            {
                "success": not result.conflicts,
                "plan_id": command.plan_id,
                "result": result,
                "conflicts": result.conflicts,
            }
        )
    if command.action == "rollback":
        result = await self.app.corrections.rollback(
            ctx,
            RollbackCorrection(
                operation_id=operation_id,
                plan_id=command.plan_id,
                reason=command.reason or "Rollback correction",
            ),
        )
        return json_object(
            {
                "success": not result.conflicts,
                "plan_id": command.plan_id,
                "result": result,
                "conflicts": result.conflicts,
            }
        )
    entity_ids: tuple[str, ...] = ()
    scope = {
        "profile": "person_profile",
        "person": "person_profile",
        "person_fact": "person_profile",
        "general": "memory",
        "chat": "memory",
    }.get(command.scope.strip().lower(), command.scope.strip().lower())
    if scope == "person_profile" and not command.edits:
        person = await select_person(
            self, ctx, ProfileInput(person_id=command.person_id, person_name=command.person_keyword)
        )
        entity_ids = (person.reference.id,)
    elif scope not in {
        "person_profile",
        "memory",
        "global",
        "all",
        "paragraph",
        "relation",
    }:
        raise DomainError("correction_scope_invalid", "Unknown correction scope")
    source_ids = None
    if command.chat_id:
        source_ids = tuple(
            source["id"]
            for source in await self.app.sources.list(ctx)
            if source["origin"].get("conversation_ref") == command.chat_id
        )
    plan = await self.app.corrections.plan(
        ctx,
        PlanCorrection(
            operation_id=operation_id,
            instruction=command.request_text,
            edits=command.edits,
            selection=RecordsQuery(
                entity_ids=entity_ids, source_ids=source_ids, limit=command.limit
            ),
        ),
    )
    return json_object(
        {
            "success": bool(plan.changes),
            "plan_id": plan.id,
            "plan": plan,
            "candidates": plan.targets,
            "operations": plan.changes,
            "questions": plan.questions,
        }
    )
