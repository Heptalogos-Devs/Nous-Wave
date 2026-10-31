"""Whole-key overrides, scoped inheritance and immutable change history."""

from collections.abc import Callable
from dataclasses import dataclass
from typing import Literal

import sqlalchemy as sa
from nous_sdk.contracts import CallContext
from nous_sdk.contracts.configuration import ConfigurationCatalog, ConfigurationDefinition
from pydantic import BaseModel, JsonValue, TypeAdapter
from sqlalchemy.ext.asyncio import AsyncConnection

from nous.platform.errors import NousError
from nous.platform.persistence import Authority, ReadView, WriteTx, metadata

configuration = sa.Table(
    "nous_configuration",
    metadata,
    sa.Column("scope", sa.String, primary_key=True),
    sa.Column("key", sa.String, primary_key=True),
    sa.Column("value", sa.JSON, nullable=False),
    sa.Column("commit_seq", sa.Integer, nullable=False),
)
history = sa.Table(
    "nous_configuration_history",
    metadata,
    sa.Column("commit_seq", sa.Integer, primary_key=True),
    sa.Column("scope", sa.String, primary_key=True),
    sa.Column("key", sa.String, primary_key=True),
    sa.Column("value", sa.JSON),
    sa.Column("previous_value", sa.JSON),
)


@dataclass(frozen=True)
class ConfigurationEntry:
    default: BaseModel
    instance_management: bool = False
    scopes: tuple[Literal["instance", "namespace", "subject"], ...] = (
        "instance",
        "namespace",
        "subject",
    )
    validator: Callable[[BaseModel], None] | None = None
    schema: Callable[[], dict[str, JsonValue]] | None = None

    def parse(self, value: object) -> BaseModel:
        parsed = type(self.default).model_validate(value)
        if self.validator is not None:
            self.validator(parsed)
        return parsed


class ConfigurationService:
    def __init__(self, authority: Authority, entries: dict[str, ConfigurationEntry]) -> None:
        self.authority = authority
        self._entries = dict(entries)
        self._defaults = {
            key: entry.parse(entry.default).model_dump(mode="json")
            for key, entry in self._entries.items()
        }

    def entry(self, key: str) -> ConfigurationEntry:
        try:
            return self._entries[key]
        except KeyError as error:
            raise NousError(
                "configuration_key_not_found",
                "Configuration key is not registered",
                status=404,
                context={"key": key},
            ) from error

    def describe(self, key: str) -> ConfigurationDefinition:
        entry = self.entry(key)
        return ConfigurationDefinition(
            key=key,
            value_schema=entry.schema()
            if entry.schema
            else type(entry.default).model_json_schema(),
            default_value=self._defaults[key],
            write_access="instance_manage" if entry.instance_management else "manage",
            scopes=entry.scopes,
        )

    def catalog(self) -> ConfigurationCatalog:
        return ConfigurationCatalog(
            items=tuple(self.describe(key) for key in sorted(self._entries))
        )

    def scopes(self, ctx: CallContext) -> tuple[str, ...]:
        scopes = ("instance", f"namespace:{ctx.namespace_id}")
        return (*scopes, f"subject:{ctx.subject_id}") if ctx.subject_id else scopes

    async def effective(self, ctx: CallContext, key: str) -> dict[str, object]:
        entry = self.entry(key)
        scopes = tuple(
            scope for scope in self.scopes(ctx) if scope.split(":", 1)[0] in entry.scopes
        )
        return await self._effective(key, scopes, ctx)

    async def effective_instance(self, key: str) -> dict[str, object]:
        entry = self.entry(key)
        if entry.scopes != ("instance",):
            raise ValueError("Instance resource resolution requires an instance-only definition")
        return await self._effective(key, ("instance",), None)

    async def _effective(
        self, key: str, scopes: tuple[str, ...], ctx: CallContext | None
    ) -> dict[str, object]:
        async with self.authority.read(ctx) as view:
            return await self.effective_in(view, ctx, key, scopes=scopes)

    async def effective_in(
        self,
        view: ReadView | WriteTx,
        ctx: CallContext | None,
        key: str,
        *,
        scopes: tuple[str, ...] | None = None,
    ) -> dict[str, object]:
        entry = self.entry(key)
        if scopes is None:
            if ctx is None:
                raise ValueError("Scoped configuration requires a context")
            scopes = tuple(
                scope for scope in self.scopes(ctx) if scope.split(":", 1)[0] in entry.scopes
            )
        value = self._defaults[key]
        origin = "default"
        commit_seq = 0
        for scope in scopes:
            row = (
                (
                    await view.connection.execute(
                        sa.select(configuration).where(
                            configuration.c.scope == scope, configuration.c.key == key
                        )
                    )
                )
                .mappings()
                .first()
            )
            if row:
                value, origin, commit_seq = row["value"], scope, int(row["commit_seq"])
        resolved = entry.parse(value).model_dump(mode="json")
        return {"key": key, "value": resolved, "origin": origin, "commit_seq": commit_seq}

    async def history(self, ctx: CallContext, key: str, limit: int = 100) -> dict[str, object]:
        self.entry(key)
        if not 1 <= limit <= 1000:
            raise ValueError("Configuration history limit must be between 1 and 1000")
        async with self.authority.read(ctx) as view:
            rows = (
                await view.connection.execute(
                    sa.select(history)
                    .where(history.c.key == key, history.c.scope.in_(self.scopes(ctx)))
                    .order_by(history.c.commit_seq.desc())
                    .limit(limit)
                )
            ).mappings()
            return {"key": key, "items": [dict(row) for row in rows], "watermark": view.commit_seq}

    async def set(
        self,
        ctx: CallContext,
        key: str,
        value: object,
        operation_id: str,
        scope: Literal["instance", "namespace", "subject"] = "instance",
    ) -> dict[str, object]:
        entry = self.entry(key)
        if scope not in entry.scopes:
            raise NousError(
                "configuration_scope_invalid",
                "Configuration does not support the selected scope",
                context={"key": key, "scope": scope, "allowed_scopes": list(entry.scopes)},
            )
        if scope == "subject" and ctx.subject_id is None:
            raise ValueError("Subject scope requires a subject")
        target = (
            "instance"
            if scope == "instance"
            else f"{scope}:{ctx.namespace_id if scope == 'namespace' else ctx.subject_id}"
        )
        parsed = None if value is None else entry.parse(value).model_dump(mode="json")
        async with self.authority.write(
            ctx,
            operation_id,
            operation_kind="configuration.set",
            scope=target,
            payload={"key": key, "value": parsed},
        ) as tx:
            if tx.replayed:
                assert tx.receipt is not None
                return TypeAdapter(dict[str, object]).validate_python(tx.receipt.result)
            return tx.result(await self.set_in(tx, ctx, key, parsed, scope))

    async def set_in(
        self,
        tx: WriteTx,
        ctx: CallContext,
        key: str,
        value: object,
        scope: Literal["instance", "namespace", "subject"] = "namespace",
    ) -> dict[str, object]:
        if tx.authority is not self.authority or tx.replayed:
            raise ValueError("Configuration changes require the owning live transaction")
        entry = self.entry(key)
        if scope not in entry.scopes:
            raise NousError(
                "configuration_scope_invalid",
                "Configuration does not support the selected scope",
                context={"key": key, "scope": scope},
            )
        if scope == "subject" and ctx.subject_id is None:
            raise ValueError("Subject scope requires a subject")
        target = (
            "instance"
            if scope == "instance"
            else f"{scope}:{ctx.namespace_id if scope == 'namespace' else ctx.subject_id}"
        )
        parsed = None if value is None else entry.parse(value).model_dump(mode="json")
        if (
            key == "namespace.quota"
            and parsed is not None
            and parsed.get("max_storage_bytes") is not None
            and tx.storage_baseline is None
            and self.authority.namespace_limits is not None
        ):
            usage = await self.authority.namespace_limits.usage_in(tx, ctx)
            tx.storage_baseline = int(str(usage["storage_bytes"]))
        applied_scopes = self.scopes(ctx)
        target_scopes = applied_scopes[: applied_scopes.index(target) + 1]
        previous = await self.effective_in(tx, ctx, key, scopes=target_scopes)
        await tx.connection.execute(
            configuration.delete().where(
                configuration.c.scope == target, configuration.c.key == key
            )
        )
        if parsed is not None:
            await tx.connection.execute(
                configuration.insert().values(
                    scope=target, key=key, value=parsed, commit_seq=tx.commit_seq
                )
            )
        await tx.connection.execute(
            history.insert().values(
                scope=target,
                key=key,
                value=parsed,
                previous_value=previous["value"],
                commit_seq=tx.commit_seq,
            )
        )
        return {"key": key, "scope": target, "value": parsed, "commit_seq": tx.commit_seq}

    async def restore_in(
        self,
        tx: WriteTx,
        ctx: CallContext,
        key: str,
        commit_seq: int | None = None,
        scope: Literal["instance", "namespace", "subject"] = "subject",
    ) -> dict[str, object]:
        """Publish a retained configuration value as a new revision at the chosen scope."""
        target = (
            "instance"
            if scope == "instance"
            else f"{scope}:{ctx.namespace_id if scope == 'namespace' else ctx.require_subject()}"
        )
        statement = sa.select(history).where(history.c.key == key, history.c.scope == target)
        if commit_seq is not None:
            statement = statement.where(history.c.commit_seq == commit_seq)
        row = (
            (await tx.connection.execute(statement.order_by(history.c.commit_seq.desc()).limit(1)))
            .mappings()
            .first()
        )
        if row is None:
            raise NousError(
                "configuration_revision_not_found",
                "Configuration history is unavailable",
                status=404,
            )
        if commit_seq is not None:
            value = row["value"]
        elif row["previous_value"] is not None:
            value = row["previous_value"]
        else:
            # Existing history predates previous-value capture. Resolve each
            # configured scope at the retained cutoff using its own history.
            value = self._defaults[key]
            for candidate in self.scopes(ctx)[: self.scopes(ctx).index(target) + 1]:
                older = (
                    await tx.connection.execute(
                        sa.select(history.c.value)
                        .where(
                            history.c.key == key,
                            history.c.scope == candidate,
                            history.c.commit_seq < int(row["commit_seq"]),
                        )
                        .order_by(history.c.commit_seq.desc())
                        .limit(1)
                    )
                ).first()
                if older is not None and older[0] is not None:
                    value = older[0]
        return await self.set_in(tx, ctx, key, value, scope)


async def migrate(connection: AsyncConnection) -> None:
    """Existing history held one key per commit; native atomic changes can hold several."""
    primary = await connection.run_sync(
        lambda sync: sa.inspect(sync).get_pk_constraint(history.name)["constrained_columns"]
    )
    if set(primary) != {"commit_seq", "scope", "key"}:
        await connection.exec_driver_sql(
            "ALTER TABLE nous_configuration_history RENAME TO nous_configuration_history_previous"
        )
        await connection.run_sync(history.create)
        await connection.exec_driver_sql(
            "INSERT INTO nous_configuration_history (commit_seq,scope,key,value) SELECT commit_seq,scope,key,value FROM nous_configuration_history_previous"
        )
        await connection.exec_driver_sql("DROP TABLE nous_configuration_history_previous")
    columns = await connection.run_sync(
        lambda sync: {column["name"] for column in sa.inspect(sync).get_columns(history.name)}
    )
    if "previous_value" not in columns:
        await connection.exec_driver_sql(
            "ALTER TABLE nous_configuration_history ADD COLUMN previous_value JSON"
        )
    from nous_sdk.contracts.namespace_policy import FormationPolicy

    legacy = (
        (
            await connection.execute(
                sa.select(configuration).where(configuration.c.key == "integrations.amemorix")
            )
        )
        .mappings()
        .all()
    )
    for row in legacy:
        old = row["value"]
        if "form_memory" not in old:
            continue
        existing = await connection.scalar(
            sa.select(configuration.c.key).where(
                configuration.c.scope == row["scope"], configuration.c.key == "formation"
            )
        )
        if existing is None:
            await connection.execute(
                configuration.insert().values(
                    scope=row["scope"],
                    key="formation",
                    value=FormationPolicy(default_enabled=bool(old["form_memory"])).model_dump(
                        mode="json"
                    ),
                    commit_seq=row["commit_seq"],
                )
            )
        await connection.execute(
            configuration.update()
            .where(configuration.c.scope == row["scope"], configuration.c.key == row["key"])
            .values(value={key: value for key, value in old.items() if key != "form_memory"})
        )
