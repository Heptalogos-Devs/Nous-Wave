"""The HTTP host owns its optional gRPC listener and closes it before the application."""

from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING

from nous_sdk.contracts import DomainError

if TYPE_CHECKING:
    from nous.application import Application


@asynccontextmanager
async def serving(app: Application) -> AsyncGenerator[None]:
    settings = app.settings.rpc
    if not settings.enabled:
        yield
        return
    try:
        from .dispatch import dispatch
        from .server import RPCServer
    except ModuleNotFoundError as error:
        raise DomainError(
            "capability_unavailable",
            "Install nous-server[amemorix-rpc] to enable the gRPC listener",
        ) from error
    server = RPCServer(
        app,
        dispatch,
        host=settings.host,
        port=settings.port,
        maximum_message_bytes=settings.maximum_message_bytes,
    )
    try:
        await server.start()
        yield
    finally:
        await server.close()
