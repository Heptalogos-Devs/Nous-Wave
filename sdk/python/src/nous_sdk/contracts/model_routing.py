"""Safe route policy, physical attempt attribution and retained outcomes."""

from typing import Literal

from pydantic import Field

from .common import Contract

SafeRejection = Literal[401, 403, 404, 405, 413, 415, 422, 429]


class ModelRoutingPolicy(Contract):
    unavailable: bool = True
    not_sent: bool = True
    rejected_statuses: tuple[SafeRejection, ...] = (401, 403, 404, 405, 413, 415, 422, 429)
    unsupported_request: bool = True
    rate_limit_retries: int = Field(default=0, ge=0, le=10)
    rate_limit_wait_seconds: float = Field(default=60, ge=0, le=300, allow_inf_nan=False)
    max_retry_after_seconds: float = Field(default=120, ge=0, le=600, allow_inf_nan=False)


class ModelCallOutcome(Contract):
    kind: Literal["not_sent", "rejected", "invalid_output", "unknown"]
    http_status: int | None = None
    provider_code: str | None = None
    provider_type: str | None = None
    parameter: str | None = None
    retry_after_seconds: float | None = Field(default=None, ge=0, allow_inf_nan=False)


class ModelRouteAttempt(Contract):
    intent_id: str
    route_id: str | None
    route_index: int
