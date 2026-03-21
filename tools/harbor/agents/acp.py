from __future__ import annotations

import asyncio
import contextlib
import json
import os
import shlex
import tempfile
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from acp import PROTOCOL_VERSION, RequestError, connect_to_agent, text_block
from acp.interfaces import Client
from acp.schema import (
    AgentMessageChunk,
    AgentPlanUpdate,
    AgentThoughtChunk,
    AllowedOutcome,
    AvailableCommandsUpdate,
    ClientCapabilities,
    CreateTerminalResponse,
    CurrentModeUpdate,
    DeniedOutcome,
    EnvVariable,
    FileSystemCapability,
    KillTerminalCommandResponse,
    PermissionOption,
    ReadTextFileResponse,
    ReleaseTerminalResponse,
    RequestPermissionResponse,
    TerminalExitStatus,
    TerminalOutputResponse,
    ToolCall,
    ToolCallProgress,
    ToolCallStart,
    UserMessageChunk,
    WaitForTerminalExitResponse,
    WriteTextFileResponse,
)
from harbor.agents.base import BaseAgent
from harbor.environments.base import BaseEnvironment, ExecResult
from harbor.models.agent.context import AgentContext


def _jsonable(value: Any) -> Any:
    if hasattr(value, "model_dump"):
        try:
            return value.model_dump(mode="json", exclude_none=True)
        except TypeError:
            return value.model_dump(exclude_none=True)
    if isinstance(value, dict):
        return {key: _jsonable(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_jsonable(item) for item in value]
    return value


def _slice_text(content: str, line: int | None, limit: int | None) -> str:
    lines = content.splitlines()
    start = max((line or 1) - 1, 0) if line is not None else 0
    end = len(lines)
    if limit is not None:
        end = min(start + limit, end)
    return "\n".join(lines[start:end])


def _choose_permission(
    options: list[PermissionOption], permission_mode: str
) -> PermissionOption | None:
    preferred_order = {
        "allow_always": ["allow_always", "allow_once"],
        "allow_once": ["allow_once", "allow_always"],
        "deny": [],
    }.get(permission_mode, ["allow_once", "allow_always"])

    for kind in preferred_order:
        for option in options:
            if option.kind == kind:
                return option
    return None


@dataclass
class TerminalState:
    command: str
    task: asyncio.Task[ExecResult]
    output: str = ""
    exit_code: int | None = None
    released: bool = False


class HarborAcpClient(Client):
    def __init__(
        self,
        *,
        environment: BaseEnvironment,
        logs_dir: Path,
        logger,
        session_cwd: str,
        permission_mode: str,
        enable_terminal: bool,
    ) -> None:
        self.environment = environment
        self.logs_dir = logs_dir
        self.logger = logger
        self.session_cwd = session_cwd
        self.permission_mode = permission_mode
        self.enable_terminal = enable_terminal

        self._updates_path = logs_dir / "session-updates.jsonl"
        self._assistant_path = logs_dir / "assistant-response.txt"
        self._assistant_chunks: list[str] = []
        self._thought_chunks: list[str] = []
        self._terminals: dict[str, TerminalState] = {}

    def assistant_text(self) -> str:
        return "".join(self._assistant_chunks).strip()

    def thought_text(self) -> str:
        return "".join(self._thought_chunks).strip()

    def _log_event(self, event_type: str, payload: Any) -> None:
        record = {"event": event_type, "payload": _jsonable(payload)}
        with self._updates_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")

    async def request_permission(
        self,
        options: list[PermissionOption],
        session_id: str,
        tool_call: ToolCall,
        **kwargs: Any,
    ) -> RequestPermissionResponse:
        self._log_event(
            "request_permission",
            {
                "session_id": session_id,
                "tool_call": tool_call,
                "options": options,
                "kwargs": kwargs,
            },
        )
        option = _choose_permission(options, self.permission_mode)
        if option is None:
            return RequestPermissionResponse(
                outcome=DeniedOutcome(outcome="cancelled")
            )
        return RequestPermissionResponse(
            outcome=AllowedOutcome(optionId=option.option_id, outcome="selected")
        )

    async def write_text_file(
        self,
        content: str,
        path: str,
        session_id: str,
        **kwargs: Any,
    ) -> WriteTextFileResponse:
        self._log_event(
            "write_text_file",
            {"session_id": session_id, "path": path, "kwargs": kwargs},
        )
        file_path = Path(path)
        if not file_path.is_absolute():
            raise RequestError.invalid_params(
                {"path": path, "reason": "path must be absolute"}
            )

        with tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8") as handle:
            handle.write(content)
            temp_path = Path(handle.name)

        try:
            await self.environment.exec(
                command=f"mkdir -p {shlex.quote(file_path.parent.as_posix())}"
            )
            await self.environment.upload_file(temp_path, file_path.as_posix())
        finally:
            temp_path.unlink(missing_ok=True)

        return WriteTextFileResponse()

    async def read_text_file(
        self,
        path: str,
        session_id: str,
        limit: int | None = None,
        line: int | None = None,
        **kwargs: Any,
    ) -> ReadTextFileResponse:
        self._log_event(
            "read_text_file",
            {
                "session_id": session_id,
                "path": path,
                "limit": limit,
                "line": line,
                "kwargs": kwargs,
            },
        )
        file_path = Path(path)
        if not file_path.is_absolute():
            raise RequestError.invalid_params(
                {"path": path, "reason": "path must be absolute"}
            )

        with tempfile.NamedTemporaryFile("r", delete=False, encoding="utf-8") as handle:
            temp_path = Path(handle.name)

        try:
            await self.environment.download_file(file_path.as_posix(), temp_path)
            content = temp_path.read_text(encoding="utf-8")
        finally:
            temp_path.unlink(missing_ok=True)

        if line is not None or limit is not None:
            content = _slice_text(content, line, limit)

        return ReadTextFileResponse(content=content)

    async def session_update(
        self,
        session_id: str,
        update: (
            UserMessageChunk
            | AgentMessageChunk
            | AgentThoughtChunk
            | ToolCallStart
            | ToolCallProgress
            | AgentPlanUpdate
            | AvailableCommandsUpdate
            | CurrentModeUpdate
        ),
        **kwargs: Any,
    ) -> None:
        self._log_event(
            "session_update",
            {"session_id": session_id, "update": update, "kwargs": kwargs},
        )
        if isinstance(update, AgentMessageChunk) and hasattr(update.content, "text"):
            self._assistant_chunks.append(update.content.text)
            self._assistant_path.write_text(
                "".join(self._assistant_chunks), encoding="utf-8"
            )
        elif isinstance(update, AgentThoughtChunk) and hasattr(update.content, "text"):
            self._thought_chunks.append(update.content.text)

    async def create_terminal(
        self,
        command: str,
        session_id: str,
        args: list[str] | None = None,
        cwd: str | None = None,
        env: list[EnvVariable] | None = None,
        output_byte_limit: int | None = None,
        **kwargs: Any,
    ) -> CreateTerminalResponse:
        if not self.enable_terminal:
            raise RequestError.method_not_found()

        argv = [command, *(args or [])]
        shell_command = shlex.join(argv)
        run_cwd = cwd or self.session_cwd
        env_dict = {item.name: item.value for item in env or []}
        terminal_id = f"term-{uuid.uuid4().hex[:8]}"

        async def _run() -> ExecResult:
            result = await self.environment.exec(
                command=shell_command,
                cwd=run_cwd,
                env=env_dict or None,
            )
            merged = (result.stdout or "") + (result.stderr or "")
            if output_byte_limit is not None:
                merged = merged[:output_byte_limit]
            state = self._terminals[terminal_id]
            state.output = merged
            state.exit_code = result.return_code
            return result

        self._log_event(
            "create_terminal",
            {
                "session_id": session_id,
                "terminal_id": terminal_id,
                "command": shell_command,
                "cwd": run_cwd,
                "env": env_dict,
                "kwargs": kwargs,
            },
        )
        self._terminals[terminal_id] = TerminalState(
            command=shell_command,
            task=asyncio.create_task(_run()),
        )
        return CreateTerminalResponse(terminalId=terminal_id)

    async def terminal_output(
        self, session_id: str, terminal_id: str, **kwargs: Any
    ) -> TerminalOutputResponse:
        state = self._terminals[terminal_id]
        self._log_event(
            "terminal_output",
            {"session_id": session_id, "terminal_id": terminal_id, "kwargs": kwargs},
        )
        exit_status = None
        if state.exit_code is not None:
            exit_status = TerminalExitStatus(exitCode=state.exit_code)
        return TerminalOutputResponse(
            output=state.output,
            truncated=False,
            exitStatus=exit_status,
        )

    async def wait_for_terminal_exit(
        self, session_id: str, terminal_id: str, **kwargs: Any
    ) -> WaitForTerminalExitResponse:
        state = self._terminals[terminal_id]
        self._log_event(
            "wait_for_terminal_exit",
            {"session_id": session_id, "terminal_id": terminal_id, "kwargs": kwargs},
        )
        with contextlib.suppress(asyncio.CancelledError):
            await state.task
        return WaitForTerminalExitResponse(exitCode=state.exit_code)

    async def release_terminal(
        self, session_id: str, terminal_id: str, **kwargs: Any
    ) -> ReleaseTerminalResponse:
        self._log_event(
            "release_terminal",
            {"session_id": session_id, "terminal_id": terminal_id, "kwargs": kwargs},
        )
        state = self._terminals.get(terminal_id)
        if state is not None:
            state.released = True
        return ReleaseTerminalResponse()

    async def kill_terminal(
        self, session_id: str, terminal_id: str, **kwargs: Any
    ) -> KillTerminalCommandResponse:
        self._log_event(
            "kill_terminal",
            {"session_id": session_id, "terminal_id": terminal_id, "kwargs": kwargs},
        )
        state = self._terminals.get(terminal_id)
        if state is not None and not state.task.done():
            state.task.cancel()
        return KillTerminalCommandResponse()


class AcpAgent(BaseAgent):
    @staticmethod
    def name() -> str:
        return "acp-bridge"

    def __init__(
        self,
        logs_dir: Path,
        model_name: str | None = None,
        backend_command: str | None = None,
        backend_args: str | None = None,
        backend_cwd: str | None = None,
        session_cwd: str = "/app",
        permission_mode: str = "allow_once",
        enable_terminal: bool = True,
        enable_filesystem: bool = True,
        extra_env: dict[str, str] | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(logs_dir=logs_dir, model_name=model_name, **kwargs)
        self.backend_command = backend_command
        self.backend_args = backend_args or ""
        self.backend_cwd = backend_cwd
        self.session_cwd = session_cwd
        self.permission_mode = permission_mode
        self.enable_terminal = enable_terminal
        self.enable_filesystem = enable_filesystem
        self.extra_env = extra_env or {}

    def version(self) -> str:
        return "0.1.0"

    async def setup(self, environment: BaseEnvironment) -> None:
        del environment

    def _resolved_backend_argv(self) -> list[str]:
        if not self.backend_command:
            raise RuntimeError("backend_command is required for AcpAgent")

        argv = [self.backend_command, *shlex.split(self.backend_args)]
        if Path(self.backend_command).name == "codex-acp" and self.model_name:
            if "model=" not in self.backend_args:
                model = self.model_name.split("/", 1)[-1]
                argv.extend(["-c", f'model="{model}"'])
        return argv

    async def run(
        self,
        instruction: str,
        environment: BaseEnvironment,
        context: AgentContext,
    ) -> None:
        argv = self._resolved_backend_argv()
        env = os.environ.copy()
        env.update(self.extra_env)

        bridge_log = self.logs_dir / "bridge.json"
        stderr_path = self.logs_dir / "backend-stderr.txt"
        stderr_handle = stderr_path.open("w", encoding="utf-8")
        client = HarborAcpClient(
            environment=environment,
            logs_dir=self.logs_dir,
            logger=self.logger,
            session_cwd=self.session_cwd,
            permission_mode=self.permission_mode,
            enable_terminal=self.enable_terminal,
        )
        proc: asyncio.subprocess.Process | None = None
        conn = None

        try:
            proc = await asyncio.create_subprocess_exec(
                *argv,
                cwd=self.backend_cwd,
                env=env,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=stderr_handle,
            )
            if proc.stdin is None or proc.stdout is None:
                raise RuntimeError("ACP backend did not expose stdio pipes")

            conn = connect_to_agent(client, proc.stdin, proc.stdout)
            init_response = await conn.initialize(
                protocol_version=PROTOCOL_VERSION,
                client_capabilities=ClientCapabilities(
                    fs=FileSystemCapability(
                        readTextFile=self.enable_filesystem,
                        writeTextFile=self.enable_filesystem,
                    ),
                    terminal=self.enable_terminal,
                ),
            )
            session = await conn.new_session(
                cwd=self.session_cwd,
                mcp_servers=[],
            )
            prompt_response = await conn.prompt(
                prompt=[text_block(instruction)],
                session_id=session.session_id,
            )

            if prompt_response.usage is not None:
                usage = prompt_response.usage
                context.n_input_tokens = usage.input_tokens
                context.n_output_tokens = usage.output_tokens
                context.n_cache_tokens = usage.cached_read_tokens

            context.metadata = {
                "backend_command": argv[0],
                "backend_args": argv[1:],
                "stop_reason": prompt_response.stop_reason,
                "session_id": session.session_id,
                "agent_info": _jsonable(init_response.agent_info)
                if init_response.agent_info
                else None,
            }

            bridge_log.write_text(
                json.dumps(
                    {
                        "argv": argv,
                        "session_cwd": self.session_cwd,
                        "initialize": _jsonable(init_response),
                        "new_session": _jsonable(session),
                        "prompt_response": _jsonable(prompt_response),
                        "assistant_text": client.assistant_text(),
                        "thought_text": client.thought_text(),
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )
        finally:
            if conn is not None:
                with contextlib.suppress(Exception):
                    await conn.close()

            if proc is not None and proc.returncode is None:
                proc.terminate()
                with contextlib.suppress(Exception):
                    await asyncio.wait_for(proc.wait(), timeout=5)
                if proc.returncode is None:
                    proc.kill()
                    with contextlib.suppress(Exception):
                        await proc.wait()

            stderr_handle.close()
