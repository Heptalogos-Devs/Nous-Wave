"""Retained model-call facts exchanged by native knowledge packages."""

from __future__ import annotations

from typing import Literal

from pydantic import Field, JsonValue

from .common import Contract


class CallOrigin(Contract):
    namespace_id: str
    subject_id: str | None
    call_id: str


class ArchivedCall(Contract):
    id: str
    origin: CallOrigin
    purpose: str
    request_hash: str
    day: str
    status: Literal["reserved", "succeeded", "failed", "purged"]
    request: dict[str, JsonValue]
    result: JsonValue = None
    usage: dict[str, JsonValue] | None = None
    error_code: str | None = None
    source_ids: tuple[str, ...] = Field(default=())


class InvocationBinding(Contract):
    key: str
    namespace_id: str | None = None
    subject_id: str | None = None
    caller_id: str | None = None
    role: str | None = None
    invocation_id: str | None = None
    call_id: str | None = None
    retired: bool = False
    reason: str | None = None
    recorded_at: str | None = None


class InvocationRestore(Contract):
    restored: int = 0
    frozen: int = 0
    opaque: tuple[InvocationBinding, ...] = ()


class RetainedModelOutput(Contract):
    call_id: str
    status: Literal["reserved", "succeeded", "failed", "purged"]
    role: str
    variant: str | None = None
    finish_reason: str | None = None
    complete: bool
    raw_text: str | None = None
    raw_json: JsonValue = None
    saved_output: JsonValue = None
    parsed_output: JsonValue = None
    validation_errors: tuple[dict[str, JsonValue], ...] = ()
