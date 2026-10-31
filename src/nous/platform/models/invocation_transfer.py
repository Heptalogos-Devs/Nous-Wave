"""Recover proven invocation identities and transfer their custody without new provider calls."""

from collections.abc import Mapping

import sqlalchemy as sa
from nous_sdk.contracts import CallContext, DomainError
from nous_sdk.contracts.model_calls import InvocationBinding, InvocationRestore
from pydantic import JsonValue, TypeAdapter
from sqlalchemy import event
from sqlalchemy.dialects.sqlite import insert
from sqlalchemy.ext.asyncio import AsyncConnection

from nous.platform.persistence import ReadView, WriteTx
from nous.platform.persistence.schema import metadata, operations

from .invocations import child, invocation_key, invocations, retire_in, retirements


async def migrate(connection: AsyncConnection) -> None:
    await connection.run_sync(_migrate)


async def recover_identities_in(connection: AsyncConnection) -> None:
    await connection.run_sync(_recover_identities)


def _migrate(connection: sa.Connection) -> None:
    added = False
    for table in (invocations, retirements):

        def columns(conn: sa.Connection, name: str) -> set[str]:
            return {column["name"] for column in sa.inspect(conn).get_columns(name)}

        existing = columns(connection, table.name)
        for name in (
            "namespace_id",
            "subject_id",
            "caller_id",
            "role",
            "invocation_id",
            *(("reason",) if table is retirements else ()),
        ):
            if name not in existing:
                connection.exec_driver_sql(
                    f'ALTER TABLE "{table.name}" ADD COLUMN "{name}" VARCHAR'
                )
                added = True
    if added:
        _recover_identities(connection)


def _recover_identities(connection: sa.Connection) -> None:
    # Namespace and subject were constrained by the original writer transaction.
    # A retirement can name a different historical caller, so its caller is never
    # inferred from the administrator who erased it.
    for table in (invocations, retirements):
        rows = (
            (
                connection.execute(
                    sa.select(
                        table.c.key,
                        operations.c.namespace_id,
                        operations.c.subject_id,
                        operations.c.caller_id,
                    )
                    .join(operations, table.c.commit_seq == operations.c.commit_seq)
                    .where(table.c.namespace_id.is_(None))
                )
            )
            .mappings()
            .all()
        )
        for row in rows:
            fields = {"namespace_id": row["namespace_id"], "subject_id": row["subject_id"]}
            if table is invocations:
                fields["caller_id"] = row["caller_id"]
            connection.execute(table.update().where(table.c.key == row["key"]).values(**fields))
    unresolved = set(
        (
            connection.execute(
                sa.select(invocations.c.key).where(invocations.c.invocation_id.is_(None))
            )
        ).scalars()
    ) | set(
        (
            connection.execute(
                sa.select(retirements.c.key).where(retirements.c.invocation_id.is_(None))
            )
        ).scalars()
    )
    if not unresolved:
        return
    counts = {
        str(row[0]): int(row[1])
        for row in (
            connection.execute(
                sa.select(invocations.c.namespace_id, sa.func.count()).group_by(
                    invocations.c.namespace_id
                )
            )
        ).all()
    }
    plans = (
        (
            connection.execute(
                sa.select(
                    *(
                        operations.c[name]
                        for name in (
                            "caller_id",
                            "namespace_id",
                            "subject_id",
                            "scope",
                            "operation_id",
                            "operation_kind",
                            "result_json",
                        )
                    )
                ).where(
                    operations.c.operation_kind.in_(
                        ("model.bind_success", "model.generation_plan", "model.embedding_plan")
                    )
                )
            )
        )
        .mappings()
        .all()
    )
    for plan in plans:
        if plan["subject_id"] is None:
            continue
        ctx = CallContext(
            caller_id=plan["caller_id"],
            namespace_id=plan["namespace_id"],
            subject_id=plan["subject_id"],
            credential_source_ids=None,
        )
        role = plan["scope"]
        directory: list[str] = []
        if plan["operation_kind"] == "model.bind_success":
            directory = [plan["operation_id"]]
        else:
            saved = TypeAdapter(JsonValue).validate_json(plan["result_json"])
            if not isinstance(saved, dict) or "$nous_operation_result" in saved:
                continue

            if plan["operation_kind"] == "model.generation_plan":
                routes = saved.get("routes")
                if isinstance(routes, list):
                    for route in routes:
                        index = route.get("index") if isinstance(route, dict) else None
                        if isinstance(index, int):
                            directory.append(child(plan["operation_id"], index))
            elif role == "embedding":
                fallbacks = saved.get("fallbacks")
                for index in range(1 + (len(fallbacks) if isinstance(fallbacks, list) else 0)):
                    prefix = (
                        plan["operation_id"] if index == 0 else child(plan["operation_id"], index)
                    )
                    directory.extend(
                        f"{prefix}:{batch}"
                        for batch in range(int(counts.get(ctx.namespace_id, 0)) + 1)
                    )
        for base in directory:
            for identity in (base, *(f"{base}:rate-limit:{index}" for index in range(1, 11))):
                if len(identity) > 512:
                    continue
                key = invocation_key(ctx, role, identity)
                if key not in unresolved:
                    continue
                fields = {
                    "namespace_id": ctx.namespace_id,
                    "subject_id": ctx.subject_id,
                    "caller_id": ctx.caller_id,
                    "role": role,
                    "invocation_id": identity,
                }
                for table in (invocations, retirements):
                    connection.execute(table.update().where(table.c.key == key).values(**fields))
                unresolved.remove(key)
        if not unresolved:
            break


async def export_in(view: ReadView | WriteTx, ctx: CallContext) -> tuple[InvocationBinding, ...]:
    from .history import imported_calls

    calls = metadata.tables["nous_model_calls"]
    custody = sa.union_all(
        sa.select(calls.c.id).where(calls.c.namespace_id == ctx.namespace_id),
        sa.select(imported_calls.c.id).where(imported_calls.c.namespace_id == ctx.namespace_id),
    ).subquery()
    result: list[InvocationBinding] = []
    for table in (invocations, retirements):
        opaque = table.c.namespace_id.is_(None)
        if table is invocations:
            opaque = sa.and_(opaque, table.c.call_id.in_(sa.select(custody.c.id)))
        else:
            opaque = sa.and_(
                opaque,
                sa.or_(
                    table.c.key.in_(
                        sa.select(invocations.c.key).where(
                            invocations.c.call_id.in_(sa.select(custody.c.id))
                        )
                    ),
                    table.c.commit_seq.in_(
                        sa.select(operations.c.commit_seq).where(
                            operations.c.namespace_id == ctx.namespace_id
                        )
                    ),
                ),
            )
        statement = sa.select(table).where(sa.or_(table.c.namespace_id == ctx.namespace_id, opaque))
        if ctx.subject_id is not None:
            statement = statement.where(table.c.subject_id == ctx.subject_id)
        for row in (await view.connection.execute(statement)).mappings():
            result.append(
                InvocationBinding.model_validate(
                    {
                        **{
                            key: row.get(key)
                            for key in (
                                "key",
                                "namespace_id",
                                "subject_id",
                                "caller_id",
                                "role",
                                "invocation_id",
                                "call_id",
                                "reason",
                                "recorded_at",
                            )
                        },
                        "retired": table is retirements,
                    }
                )
            )
    return tuple(result)


async def restore_in(
    tx: WriteTx,
    ctx: CallContext,
    bindings: tuple[InvocationBinding, ...],
    *,
    subject_mapping: Mapping[str, str],
    caller_mapping: Mapping[str, str],
    call_mapping: Mapping[str, str],
    preserve_opaque: bool = False,
    source_namespace_id: str | None = None,
) -> InvocationRestore:
    restored = frozen = 0
    opaque: list[InvocationBinding] = []
    if ctx.namespace_id != tx.ctx.namespace_id:
        raise DomainError(
            "invocation_archive_invalid", "Invocation custody must stay in the receiving namespace"
        )
    for binding in bindings:
        if None in (
            binding.namespace_id,
            binding.subject_id,
            binding.caller_id,
            binding.role,
            binding.invocation_id,
        ):
            opaque.append(binding)
            if preserve_opaque:
                if (
                    source_namespace_id != ctx.namespace_id
                    or (
                        binding.namespace_id is not None
                        and binding.namespace_id != ctx.namespace_id
                    )
                    or (
                        binding.subject_id is not None
                        and subject_mapping.get(binding.subject_id) != binding.subject_id
                    )
                ):
                    raise DomainError(
                        "invocation_archive_invalid",
                        "Opaque identities can only retain their original installation scope",
                    )
                call = call_mapping.get(binding.call_id or "")
                if not binding.retired and call is None:
                    continue
                if call is not None:
                    from .history import imported_calls

                    calls = metadata.tables["nous_model_calls"]
                    retained = await tx.connection.scalar(
                        sa.select(calls.c.id).where(
                            calls.c.id == call, calls.c.namespace_id == ctx.namespace_id
                        )
                    )
                    if retained is None:
                        retained = await tx.connection.scalar(
                            sa.select(imported_calls.c.id).where(
                                imported_calls.c.id == call,
                                imported_calls.c.namespace_id == ctx.namespace_id,
                            )
                        )
                    if retained is None:
                        raise DomainError(
                            "invocation_call_unavailable",
                            "Opaque binding requires a call already held in the receiving installation",
                        )
                    existing = await tx.connection.scalar(
                        sa.select(invocations.c.call_id).where(invocations.c.key == binding.key)
                    )
                    if existing is not None and existing != call:
                        raise DomainError(
                            "model_invocation_conflict", "Opaque key already names a different call"
                        )
                    await tx.connection.execute(
                        insert(invocations)
                        .values(
                            key=binding.key,
                            call_id=call,
                            commit_seq=tx.commit_seq,
                            **{
                                name: getattr(binding, name)
                                for name in (
                                    "namespace_id",
                                    "subject_id",
                                    "caller_id",
                                    "role",
                                    "invocation_id",
                                )
                            },
                        )
                        .on_conflict_do_nothing(index_elements=["key"])
                    )
                    restored += 1
                await tx.connection.execute(
                    insert(retirements)
                    .values(
                        key=binding.key,
                        commit_seq=tx.commit_seq,
                        recorded_at=binding.recorded_at or tx.recorded_at,
                        reason=binding.reason if binding.retired else "opaque_restored",
                        **{
                            name: getattr(binding, name)
                            for name in (
                                "namespace_id",
                                "subject_id",
                                "caller_id",
                                "role",
                                "invocation_id",
                            )
                        },
                    )
                    .on_conflict_do_nothing(index_elements=["key"])
                )
                frozen += 1
            continue
        assert (
            binding.namespace_id is not None
            and binding.subject_id is not None
            and binding.caller_id is not None
            and binding.role is not None
            and binding.invocation_id is not None
        )
        original = CallContext(
            caller_id=binding.caller_id,
            namespace_id=binding.namespace_id,
            subject_id=binding.subject_id,
            credential_source_ids=None,
        )
        if invocation_key(original, binding.role, binding.invocation_id) != binding.key:
            raise DomainError(
                "invocation_archive_invalid", "Invocation identity does not match its retained key"
            )
        subject = subject_mapping.get(binding.subject_id)
        caller = caller_mapping.get(binding.caller_id)
        if subject is None or caller is None:
            opaque.append(binding)
            continue
        receiving = ctx.model_copy(update={"subject_id": subject, "caller_id": caller})
        if ctx.subject_id is not None and subject != ctx.subject_id:
            raise DomainError(
                "invocation_archive_invalid", "Invocation mapping exceeds the receiving subject"
            )
        changed = (receiving.namespace_id, subject, caller) != (
            binding.namespace_id,
            binding.subject_id,
            binding.caller_id,
        )
        if changed or binding.retired:
            await retire_in(
                tx,
                receiving,
                binding.role,
                binding.invocation_id,
                reason="subject_migrated" if changed else binding.reason or "source_purged",
            )
            frozen += 1
            continue
        call = call_mapping.get(binding.call_id or "")
        if call is None:
            opaque.append(binding)
            continue
        from .history import imported_calls

        calls = metadata.tables["nous_model_calls"]

        present = await tx.connection.scalar(
            sa.select(calls.c.id).where(
                calls.c.id == call,
                calls.c.namespace_id == ctx.namespace_id,
                calls.c.subject_id == subject,
            )
        )
        if present is None:
            present = await tx.connection.scalar(
                sa.select(imported_calls.c.id).where(
                    imported_calls.c.id == call,
                    imported_calls.c.namespace_id == ctx.namespace_id,
                    imported_calls.c.subject_id == subject,
                )
            )
        if present is None:
            raise DomainError(
                "invocation_call_unavailable",
                "The invocation has no retained call in its receiving subject",
            )
        existing = await tx.connection.scalar(
            sa.select(invocations.c.call_id).where(invocations.c.key == binding.key)
        )
        if existing is not None and existing != call:
            raise DomainError(
                "model_invocation_conflict",
                "The receiving invocation already names a different call",
            )
        await tx.connection.execute(
            insert(invocations)
            .values(
                key=binding.key,
                call_id=call,
                namespace_id=ctx.namespace_id,
                subject_id=subject,
                caller_id=caller,
                role=binding.role,
                invocation_id=binding.invocation_id,
                commit_seq=tx.commit_seq,
            )
            .on_conflict_do_nothing(index_elements=[invocations.c.key])
        )
        restored += 1
    return InvocationRestore(restored=restored, frozen=frozen, opaque=tuple(opaque))


def _after_create(target: sa.MetaData, connection: sa.Connection, **kwargs: object) -> None:
    _migrate(connection)


event.listen(metadata, "after_create", _after_create)
