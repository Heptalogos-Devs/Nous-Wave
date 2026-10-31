"""Shared Namespace/Subject read admission; management mutation stays with IdentityService."""

import sqlalchemy as sa
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.identity import IdentityPurpose, Namespace, Subject

from nous.platform.persistence.authority import ReadView, WriteTx

from .schema import namespaces, subjects


def normalise_capabilities(values: list[str] | tuple[str, ...]) -> list[str]:
    return list(dict.fromkeys("ego" if value == "self" else value for value in values))


def _available(value: Namespace | Subject, kind: str, purpose: IdentityPurpose) -> None:
    if purpose == "management":
        return
    if isinstance(value, Namespace) and value.purging:
        raise DomainError("namespace_purging", "Namespace permanent removal is in progress")
    if value.deleted:
        raise DomainError(kind + "_deleted", "Identity is in the recycle bin", id=value.id)
    if not value.enabled:
        raise DomainError(kind + "_disabled", "Identity is disabled", id=value.id)


async def require_namespace_in(
    view: ReadView | WriteTx, ctx: CallContext, *, purpose: IdentityPurpose = "active"
) -> Namespace:
    row = (
        (
            await view.connection.execute(
                sa.select(namespaces).where(namespaces.c.id == ctx.namespace_id)
            )
        )
        .mappings()
        .first()
    )
    if row is None:
        raise DomainError("namespace_not_found", "Namespace does not exist")
    value = Namespace.model_validate(row)
    _available(value, "namespace", purpose)
    return value


async def require_subject_in(
    view: ReadView | WriteTx,
    ctx: CallContext,
    capability: str | None = None,
    *,
    purpose: IdentityPurpose = "active",
):
    await require_namespace_in(view, ctx, purpose=purpose)
    row = (
        (
            await view.connection.execute(
                sa.select(subjects).where(
                    subjects.c.id == ctx.subject_id, subjects.c.namespace_id == ctx.namespace_id
                )
            )
        )
        .mappings()
        .first()
    )
    if row is None:
        raise DomainError("subject_not_found", "Subject does not exist in this namespace")
    subject = {**dict(row), "capabilities": normalise_capabilities(row["capabilities"])}
    _available(Subject.model_validate(subject), "subject", purpose)
    if capability and capability not in subject["capabilities"]:
        raise DomainError("capability_unavailable", f"Subject does not provide {capability}")
    return subject
