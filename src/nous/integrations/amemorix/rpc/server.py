"""grpc.aio owns transport framing; registered handlers call native application operations."""

from __future__ import annotations

import logging
from collections.abc import Awaitable, Callable
from functools import partial
from typing import TYPE_CHECKING

import grpc
from google.protobuf import any_pb2, json_format, message_factory
from google.protobuf.descriptor import MethodDescriptor
from google.protobuf.message import Message
from google.rpc import status_pb2
from grpc_health.v1 import health, health_pb2, health_pb2_grpc
from grpc_status import rpc_status
from nous_sdk.contracts import DomainError
from nous_sdk.types import JsonObject
from pydantic import TypeAdapter, ValidationError

from nous.platform.errors import NousError
from nous.platform.persistence import OperationConflict

from ._protocol import auth_pb2, backup_pb2, common_pb2, job_pb2, memory_pb2, namespace_pb2
from .context import RPCContext, object_field, text_field

if TYPE_CHECKING:
    from nous.application import Application

RPCHandler = Callable[[str, str, JsonObject, RPCContext], Awaitable[JsonObject]]
_JSON = TypeAdapter(JsonObject)
logger = logging.getLogger(__name__)


def serialize_response(response: Message) -> bytes:
    return response.SerializeToString()


def public_error(error: Exception) -> tuple[str, grpc.StatusCode, str, JsonObject]:
    if isinstance(error, DomainError):
        code = error.code
        details = _JSON.validate_python(TypeAdapter(object).dump_python(error.details, mode="json"))
        details["domain_code"] = code
        if code in {"credential_invalid", "credential_expired"}:
            return "unauthorized", grpc.StatusCode.UNAUTHENTICATED, str(error), details
        if code.endswith("_forbidden") or code == "forbidden":
            return "forbidden", grpc.StatusCode.PERMISSION_DENIED, str(error), details
        if code == "namespace_not_found":
            return code, grpc.StatusCode.NOT_FOUND, str(error), details
        if "not_found" in code or code == "source_unavailable":
            return "not_found", grpc.StatusCode.NOT_FOUND, str(error), details
        if "conflict" in code:
            return "conflict", grpc.StatusCode.ALREADY_EXISTS, str(error), details
        if code in {"capability_unavailable", "model_unavailable"}:
            return "capability_unavailable", grpc.StatusCode.UNAVAILABLE, str(error), details
        if code.endswith(("_disabled", "_deleted")):
            return (
                "capability_unavailable",
                grpc.StatusCode.FAILED_PRECONDITION,
                str(error),
                details,
            )
        return "invalid_argument", grpc.StatusCode.INVALID_ARGUMENT, str(error), details
    if isinstance(error, NousError):
        status = {
            401: grpc.StatusCode.UNAUTHENTICATED,
            403: grpc.StatusCode.PERMISSION_DENIED,
            404: grpc.StatusCode.NOT_FOUND,
            409: grpc.StatusCode.ALREADY_EXISTS,
            429: grpc.StatusCode.RESOURCE_EXHAUSTED,
        }.get(
            error.status,
            grpc.StatusCode.UNAVAILABLE
            if error.status >= 500
            else grpc.StatusCode.INVALID_ARGUMENT,
        )
        return (
            "capability_unavailable",
            status,
            str(error),
            {"domain_code": error.code, **error.context},
        )
    if isinstance(error, OperationConflict):
        return "conflict", grpc.StatusCode.ALREADY_EXISTS, str(error), {}
    if isinstance(error, ValidationError | ValueError):
        return "invalid_argument", grpc.StatusCode.INVALID_ARGUMENT, str(error), {}
    logger.exception("A-Memorix RPC failed", exc_info=error)
    return (
        "internal_error",
        grpc.StatusCode.INTERNAL,
        "The service could not complete the operation",
        {},
    )


class RPCServer:
    def __init__(
        self,
        app: Application,
        handler: RPCHandler,
        *,
        host: str = "127.0.0.1",
        port: int = 7442,
        maximum_message_bytes: int = 16 * 1024 * 1024,
    ) -> None:
        self.app, self.handler, self.host, self.port = app, handler, host, port
        self.server = grpc.aio.server(
            options=(
                ("grpc.max_receive_message_length", maximum_message_bytes),
                ("grpc.max_send_message_length", maximum_message_bytes),
            )
        )
        self.health = health.aio.HealthServicer()
        health_pb2_grpc.add_HealthServicer_to_server(self.health, self.server)
        self.services: list[str] = []
        for module in (auth_pb2, namespace_pb2, memory_pb2, job_pb2, backup_pb2):
            for service in module.DESCRIPTOR.services_by_name.values():
                methods: dict[str, grpc.RpcMethodHandler[Message, Message]] = {}
                for method in service.methods:
                    request_type = message_factory.GetMessageClass(method.input_type)
                    methods[method.name] = grpc.unary_unary_rpc_method_handler(
                        partial(self._invoke, service.full_name, method),
                        request_deserializer=request_type.FromString,
                        response_serializer=serialize_response,
                    )
                self.server.add_generic_rpc_handlers(
                    (grpc.method_handlers_generic_handler(service.full_name, methods),)
                )
                self.services.append(service.full_name)

    async def _invoke(
        self,
        service: str,
        method: MethodDescriptor,
        request: Message,
        context: grpc.aio.ServicerContext[Message, Message],
    ) -> Message:
        values = _JSON.validate_python(
            json_format.MessageToDict(request, preserving_proto_field_name=True)
        )
        received = context.invocation_metadata() or ()
        metadata = {key.lower(): value for key, value in received if isinstance(value, str)}
        try:
            authorizations = [value for key, value in received if key.lower() == "authorization"]
            if len(authorizations) != 1 or not isinstance(authorizations[0], str):
                raise DomainError("credential_invalid", "Exactly one Bearer credential is required")
            scheme, separator, token = authorizations[0].partition(" ")
            if not separator or scheme.lower() != "bearer" or not token.strip():
                raise DomainError("credential_invalid", "Authorization must use the Bearer scheme")
            actor = await self.app.credentials.authenticate(token.strip())
            result = await self.handler(
                service.rsplit(".", 1)[-1],
                method.name,
                values,
                RPCContext(self.app, actor, metadata),
            )
            response_type = message_factory.GetMessageClass(method.output_type)
            return json_format.ParseDict(result, response_type())
        except Exception as error:
            code, status, message, details = public_error(error)
            supplied = object_field(values, "context")
            detail = common_pb2.ErrorDetail(
                code=code,
                request_id=text_field(supplied, "request_id") or metadata.get("x-request-id", ""),
                trace_id=text_field(supplied, "trace_id") or metadata.get("x-trace-id", ""),
                message=message,
            )
            json_format.ParseDict(details, detail.details)
            packed = any_pb2.Any()
            packed.Pack(detail)
            wire = status_pb2.Status(code=status.value[0], message=message, details=[packed])
            status_value = rpc_status.to_status(wire)
            await context.abort(
                status_value.code,
                status_value.details,
                trailing_metadata=status_value.trailing_metadata,
            )
            raise RuntimeError("RPC abort returned") from error

    async def start(self) -> None:
        bound = self.server.add_insecure_port(f"{self.host}:{self.port}")
        if not bound:
            raise OSError("The configured RPC address could not be bound")
        self.port = bound
        for service in ("", *self.services):
            await self.health.set(service, health_pb2.HealthCheckResponse.SERVING)
        await self.server.start()

    async def close(self) -> None:
        await self.health.enter_graceful_shutdown()
        await self.server.stop(5)
