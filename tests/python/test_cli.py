"""The CLI uses real HTTP, strict JSON and protected one-time credential files."""

import asyncio
import json
import os
import socket
import subprocess
import sys
import time
from pathlib import Path

import httpx
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from nous_sdk import NousClient
from opentelemetry.instrumentation.utils import suppress_http_instrumentation


def test_installed_default_instance_is_independent_of_cwd_and_keeps_overrides(tmp_path):
    command = [sys.executable, "-I", "-c", "from nous.transports.cli import main; main()"]
    environment = os.environ.copy()
    for name in ("NOUS_INSTANCE", "NOUS_CREDENTIAL_FILE"):
        environment.pop(name, None)
    environment.update(
        LOCALAPPDATA=str(tmp_path / "data"),
        XDG_DATA_HOME=str(tmp_path / "data"),
        HOME=str(tmp_path / "home"),
    )
    first, second = tmp_path / "first", tmp_path / "second"
    first.mkdir()
    second.mkdir()

    def invoke_at(directory, *arguments, env=environment):
        result = subprocess.run(
            [*command, "--json", *arguments],
            cwd=directory,
            env=env,
            check=True,
            capture_output=True,
            encoding="utf-8",
            timeout=40,
        )
        return json.loads(result.stdout)

    initialized = invoke_at(first, "init")
    expected = Path(initialized["instance"])
    assert expected.is_absolute() and expected.name == "instance"
    assert expected.is_relative_to(tmp_path)
    credential = Path(initialized["credential_file"]).read_bytes()
    assert invoke_at(second, "init") == initialized
    assert Path(initialized["credential_file"]).read_bytes() == credential
    configured = subprocess.run(
        [
            sys.executable,
            "-I",
            "-c",
            "from nous.platform.configuration import Settings; print(Settings().instance)",
        ],
        cwd=second,
        env=environment,
        check=True,
        capture_output=True,
        encoding="utf-8",
        timeout=20,
    )
    assert Path(configured.stdout.strip()) == expected
    env_instance, flag_instance = tmp_path / "environment", tmp_path / "explicit"
    overridden = {**environment, "NOUS_INSTANCE": str(env_instance)}
    assert Path(invoke_at(first, "init", env=overridden)["instance"]) == env_instance
    assert (
        Path(
            invoke_at(second, "--instance", str(flag_instance), "init", env=overridden)["instance"]
        )
        == flag_instance
    )
    assert (
        Path(
            invoke_at(second, "init", "--instance", str(flag_instance), env=overridden)["instance"]
        )
        == flag_instance
    )
    assert Path(initialized["credential_file"]).read_bytes() == credential

    # An existing installation keeps its previous data location without moving it.
    legacy_env = {
        **environment,
        "LOCALAPPDATA": str(tmp_path / "legacy-data"),
        "XDG_DATA_HOME": str(tmp_path / "legacy-data"),
        "HOME": str(tmp_path / "legacy-home"),
    }
    parent = (
        tmp_path / "legacy-home/Library/Application Support"
        if sys.platform == "darwin"
        else tmp_path / "legacy-data"
    )
    legacy = (
        parent / ("NousWave" if sys.platform in {"win32", "darwin"} else "nous-wave") / "instance"
    )
    previous = invoke_at(first, "init", "--instance", str(legacy), env=legacy_env)
    assert invoke_at(second, "init", env=legacy_env) == previous

    with socket.socket() as available:
        available.bind(("127.0.0.1", 0))
        port = available.getsockname()[1]
    url = f"http://127.0.0.1:{port}"
    with (tmp_path / "default-host.log").open("w", encoding="utf-8") as log:
        host = subprocess.Popen(
            [*command, "serve", "--no-rpc", "--port", str(port)],
            cwd=first,
            env=environment,
            stdout=log,
            stderr=log,
        )
        headers = {"Authorization": "Bearer " + credential.decode().strip()}
        try:
            deadline = time.monotonic() + 40
            while True:
                try:
                    response = httpx.get(url + "/api/status", headers=headers, timeout=1)
                    if response.is_success:
                        break
                except httpx.HTTPError:
                    pass
                assert host.poll() is None and time.monotonic() < deadline
                time.sleep(0.1)
            assert Path(response.json()["instance"]) == expected
            assert Path(invoke_at(second, "--url", url, "status")["instance"]) == expected
            assert (expected / "data/authority.sqlite").is_file()
            assert not (first / "temp/instances/default").exists()
            assert not (second / "temp/instances/default").exists()
        finally:
            if host.poll() is None:
                try:
                    httpx.post(url + "/api/stop", headers=headers, timeout=5)
                    host.wait(timeout=25)
                except httpx.HTTPError, subprocess.TimeoutExpired:
                    host.terminate()
                    host.wait(timeout=10)


def test_cli_literal_query_and_revocable_readonly_token(tmp_path):
    command = [sys.executable, "-c", "from nous.transports.cli import main; main()"]
    instance = tmp_path / "instance"
    with socket.socket() as available:
        available.bind(("127.0.0.1", 0))
        port = available.getsockname()[1]
    connection = ["--instance", str(instance), "--url", f"http://127.0.0.1:{port}"]

    def invoke(*arguments, credential=None, success=True, human=False):
        options = connection.copy()
        if credential is not None:
            options += ["--credential-file", str(credential)]
        if not human:
            options += ["--json"]
        response = subprocess.run(
            [*command, *options, *arguments], capture_output=True, encoding="utf-8", timeout=40
        )
        assert (response.returncode == 0) == success, (response.stdout, response.stderr)
        if human:
            return response.stdout
        result = json.loads(response.stdout)
        assert isinstance(result, dict)
        return result

    initialized = invoke("init")
    assert "credential_file" in initialized
    with (tmp_path / "host.log").open("w", encoding="utf-8") as log:
        host = subprocess.Popen(
            [*command, *connection, "serve", "--port", str(port)], stdout=log, stderr=log
        )
        try:
            deadline = time.monotonic() + 40
            while True:
                status = subprocess.run(
                    [*command, *connection, "--json", "status"],
                    capture_output=True,
                    encoding="utf-8",
                    timeout=10,
                )
                if status.returncode == 0:
                    break
                assert host.poll() is None and time.monotonic() < deadline, (
                    tmp_path / "host.log"
                ).read_text(encoding="utf-8")
                time.sleep(0.1)
            namespace = invoke("namespace", "create", "CLI")["id"]
            subject = invoke("subject", "create", namespace)["id"]

            async def streaming_request_isolation():
                async with NousClient(
                    f"http://127.0.0.1:{port}",
                    credential_file=instance / "credentials/admin.token",
                ) as api:
                    observed = []

                    async def capture(response):
                        observed.append(
                            (
                                response.request.headers.get("traceparent"),
                                response.headers["x-nous-trace-id"],
                            )
                        )

                    api.transport.http.event_hooks = {"response": [capture]}
                    chunk = b"No model stream bytes." * 4096

                    async def chunks():
                        for _ in range(4):
                            yield chunk
                            await asyncio.sleep(0)

                    with suppress_http_instrumentation():
                        await api.upload_stream(
                            namespace,
                            chunks(),
                            byte_len=4 * len(chunk),
                            owner_subject_id=subject,
                            media_type="text/plain",
                        )
                        await api.request("GET", "/api/status")
                        await api.request("GET", "/api/status")
                        assert all(parent is None for parent, _ in observed)
                        assert len({identity for _, identity in observed}) == 3
                        headers = await api.transport.headers()
                        headers["traceparent"] = "00-" + "a" * 32 + "-" + "b" * 16 + "-01"
                        response = await api.transport.http.get("/api/status", headers=headers)
                        assert response.headers["x-nous-trace-id"] == "a" * 32

            asyncio.run(streaming_request_isolation())
            document = tmp_path / "notes.txt"
            document.write_text(
                "Rollback preserves atomic transactions. 回滚保护原子性。", encoding="utf-8"
            )
            invoke("import", namespace, subject, str(document), "--no-llm", "--vectors", "disabled")
            assert len(invoke("query", namespace, subject, "Rollback")["hits"]) == 1
            rendered = invoke(
                "--locale", "zh-CN", "query", namespace, subject, "Rollback", human=True
            )
            assert "命中" in rendered and "回滚保护原子性" in rendered
            config = tmp_path / "credential.json"
            config.write_text(
                json.dumps(
                    {
                        "name": "Reader",
                        "namespace_ids": [namespace],
                        "subject_ids": [subject],
                        "capabilities": ["read"],
                    }
                ),
                encoding="utf-8",
            )
            token_file = tmp_path / "reader.token"
            created = invoke("credentials", "create", str(config), "--token-out", str(token_file))
            assert "token" not in created
            if os.name != "nt":
                assert token_file.stat().st_mode & 0o777 == 0o600
            identity = invoke("credentials", "whoami", credential=token_file)
            assert identity["credential_id"] == created["record"]["id"]
            assert invoke("credentials", "history", created["record"]["id"])["count"] == 1
            rotation = tmp_path / "rotation.json"
            rotation.write_text(
                json.dumps(
                    {"operation_id": "cli-rotate", "expected_revision": 1, "reason": "CLI rollover"}
                ),
                encoding="utf-8",
            )
            invoke(
                "credentials",
                "rotate",
                created["record"]["id"],
                str(rotation),
                "--token-out",
                str(token_file),
                success=False,
            )
            assert invoke("credentials", "read", created["record"]["id"])["revision"] == 1
            replacement = tmp_path / "replacement.token"
            rotated = invoke(
                "credentials",
                "rotate",
                created["record"]["id"],
                str(rotation),
                "--token-out",
                str(replacement),
            )
            assert "token" not in rotated and rotated["record"]["revision"] == 2
            rejected = invoke(
                "query", namespace, subject, "Rollback", credential=token_file, success=False
            )
            assert rejected["error"]["code"] == "credential_invalid"
            replay_target = tmp_path / "replay.token"
            replay = invoke(
                "credentials",
                "rotate",
                created["record"]["id"],
                str(rotation),
                "--token-out",
                str(replay_target),
            )
            assert replay["token"] is None and not replay_target.exists()
            token_file = replacement
            assert invoke("credentials", "history", created["record"]["id"])["count"] == 2
            assert (
                len(invoke("query", namespace, subject, "Rollback", credential=token_file)["hits"])
                == 1
            )
            denied = invoke("subject", "create", namespace, credential=token_file, success=False)
            assert "forbidden" in denied["error"]["code"].lower()

            async def mcp_protocol():
                for readonly in (False, True):
                    parameters = StdioServerParameters(
                        command=command[0],
                        args=[
                            *command[1:],
                            *connection,
                            "mcp",
                            "--namespace-id",
                            namespace,
                            "--subject-id",
                            subject,
                            "--credential-file",
                            str(token_file if readonly else instance / "credentials/admin.token"),
                        ],
                    )
                    with (tmp_path / "mcp.log").open("a", encoding="utf-8") as diagnostics:
                        async with stdio_client(parameters, errlog=diagnostics) as (reader, writer):
                            async with ClientSession(
                                reader, writer, read_timeout_seconds=15
                            ) as session:
                                initialized = await session.initialize()
                                assert initialized.server_info.name == "Nous"
                                tools = await session.list_tools()
                                assert {"manage_sources", "list_jobs", "query_cognition"} <= {
                                    tool.name for tool in tools.tools
                                }
                                arguments = {
                                    "command": {
                                        "inputs": [{"text": "MCP protocol durability."}],
                                        "llm_enabled": False,
                                        "vectors": "disabled",
                                    }
                                }
                                admitted = await session.call_tool("admit_documents", arguments)
                                assert admitted.is_error == readonly
                                found = await session.call_tool(
                                    "query_cognition", {"command": {"text": "protocol"}}
                                )
                                assert not found.is_error
                                assert len(found.structured_content["hits"]) == 1
                                wrong = await session.call_tool(
                                    "manage_sources",
                                    {"action": "list", "command": {"subject_id": "different"}},
                                )
                                assert wrong.is_error
                                manual = await session.read_resource("nous://query/manual")
                                assert manual.contents

            asyncio.run(mcp_protocol())
            assert len(invoke("query", namespace, subject, "protocol")["hits"]) == 1
            assert invoke("jobs", "page", namespace, "--subject-id", subject)["items"] == []
            invoke("credentials", "revoke", created["record"]["id"])
            revoked = invoke(
                "query", namespace, subject, "Rollback", credential=token_file, success=False
            )
            assert revoked["error"]["code"] == "credential_invalid"
            invalid = invoke("unknown-command", success=False)
            assert invalid["error"]["code"] == "InvalidArgument"
            assert invoke("source", success=False)["error"]["code"] == "InvalidArgument"
            assert invoke("model-calls", namespace, "--subject-id", subject)["count"] == 0
        finally:
            host.terminate()
            try:
                host.wait(timeout=25)
            except subprocess.TimeoutExpired:
                host.kill()
                host.wait(timeout=5)
