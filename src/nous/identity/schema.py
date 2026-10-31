"""Identity facts live in the same Authority database as their retained data."""

from sqlalchemy import (
    JSON,
    Boolean,
    Column,
    Connection,
    ForeignKey,
    Integer,
    String,
    Table,
    inspect,
    text,
)
from sqlalchemy.ext.asyncio import AsyncConnection

from nous.platform.persistence.schema import metadata

namespaces = Table(
    "namespaces",
    metadata,
    Column("id", String, primary_key=True),
    Column("name", String, nullable=False),
    Column("enabled", Boolean, nullable=False, server_default=text("1")),
    Column("deleted", Boolean, nullable=False, server_default=text("0")),
    Column("revision", Integer, nullable=False, server_default=text("1")),
    Column("commit_seq", Integer, nullable=False, server_default=text("0")),
    Column("created_at", String),
    Column("updated_at", String),
    Column("memory_subject_id", String),
    Column("purging", Boolean, nullable=False, server_default=text("0")),
)
subjects = Table(
    "subjects",
    metadata,
    Column("id", String, primary_key=True),
    Column("namespace_id", String, nullable=False, index=True),
    Column("name", String, nullable=False),
    Column("capabilities", JSON, nullable=False),
    Column("commit_seq", Integer, nullable=False),
    Column("enabled", Boolean, nullable=False, server_default=text("1")),
    Column("deleted", Boolean, nullable=False, server_default=text("0")),
    Column("revision", Integer, nullable=False, server_default=text("1")),
    Column("created_at", String),
    Column("updated_at", String),
)
subject_bindings = Table(
    "subject_bindings",
    metadata,
    Column("namespace_id", String, ForeignKey("namespaces.id"), primary_key=True),
    Column("caller_id", String, primary_key=True),
    Column("external_id", String, primary_key=True),
    Column("subject_id", String, ForeignKey("subjects.id"), nullable=False),
    Column("commit_seq", Integer, nullable=False),
)

retired = Table(
    "identity_retirements",
    metadata,
    Column("kind", String, primary_key=True),
    Column("id", String, primary_key=True),
    Column("namespace_id", String, nullable=False, index=True),
    Column("purged_at", String, nullable=False),
    Column("commit_seq", Integer, nullable=False),
)


def _columns(connection: Connection, table: str) -> set[str]:
    return {column["name"] for column in inspect(connection).get_columns(table)}


async def migrate(connection: AsyncConnection) -> None:
    # create_all handles new databases; SQLite ALTER retains existing identities and rows.
    for table in ("namespaces", "subjects"):
        columns = await connection.run_sync(_columns, table)
        additions = {
            "enabled": "BOOLEAN NOT NULL DEFAULT 1",
            "deleted": "BOOLEAN NOT NULL DEFAULT 0",
            "revision": "INTEGER NOT NULL DEFAULT 1",
            "created_at": "VARCHAR",
            "updated_at": "VARCHAR",
        }
        if table == "namespaces":
            additions["commit_seq"] = "INTEGER NOT NULL DEFAULT 0"
            additions["memory_subject_id"] = "VARCHAR"
            additions["purging"] = "BOOLEAN NOT NULL DEFAULT 0"
        for name, declaration in additions.items():
            if name not in columns:
                await connection.exec_driver_sql(
                    f'ALTER TABLE "{table}" ADD COLUMN "{name}" {declaration}'
                )
