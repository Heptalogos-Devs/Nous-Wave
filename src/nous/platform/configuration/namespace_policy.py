"""Typed namespace policies consumed by the actual domain owners."""

import sqlalchemy as sa
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.namespace_policy import FormationPolicy, NamespaceFeatures, NamespaceQuota
from pydantic import BaseModel

from nous.platform.persistence import ReadView, WriteTx

from .service import ConfigurationEntry, configuration


def entries() -> dict[str, ConfigurationEntry]:
    return {
        "namespace.features": ConfigurationEntry(NamespaceFeatures(), scopes=("namespace",)),
        "namespace.quota": ConfigurationEntry(
            NamespaceQuota(), instance_management=True, scopes=("namespace",)
        ),
        "formation": ConfigurationEntry(FormationPolicy()),
    }


async def policy_in[P: BaseModel](
    view: ReadView | WriteTx, ctx: CallContext, key: str, model: type[P]
) -> P:
    value = None
    scopes = (
        ("instance", f"namespace:{ctx.namespace_id}", f"subject:{ctx.subject_id}")
        if key == "formation"
        else (f"namespace:{ctx.namespace_id}",)
    )
    for scope in scopes:
        selected = await view.connection.scalar(
            sa.select(configuration.c.value).where(
                configuration.c.scope == scope, configuration.c.key == key
            )
        )
        if selected is not None:
            value = selected
    return model.model_validate(value if value is not None else {})


async def require_feature_in(view: ReadView | WriteTx, ctx: CallContext, feature: str) -> None:
    features = await policy_in(view, ctx, "namespace.features", NamespaceFeatures)
    if not getattr(features, feature):
        raise DomainError(
            "namespace_feature_disabled", "Namespace feature is disabled", feature=feature
        )
