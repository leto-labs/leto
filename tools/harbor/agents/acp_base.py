from __future__ import annotations

import asyncio
import contextlib
import json
import os
import shlex
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Any

from acp import PROTOCOL_VERSION, connect_to_agent, text_block
from acp.schema import ClientCapabilities, FileSystemCapability
from harbor.agents.base import BaseAgent
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext
from jinja2 import Environment

from .acp_client import HarborAcpClient, _jsonable


class BaseAcpAgent(BaseAgent, ABC):
    DEFAULT_STDIO_BUFFER_LIMIT_BYTES = 50 * 1024 * 1024
    # BaseAcpAgent copies host artifacts into this in-container staging
    # directory by calling Harbor environment.upload_file(...). These are not
    # bind mounts and are not the original host files.
    DEFAULT_STAGING_DIR = "/installed-agent/staged"
    DEFAULT_SETUP_SCRIPT_PATH = "/installed-agent/install.sh"

    def __init__(
        self,
        logs_dir: Path,
        model_name: str | None = None,
        backend_artifact_path: str | None = None,
        backend_args: str = "",
        install_dir: str = DEFAULT_STAGING_DIR,
        session_cwd: str = "/app",
        permission_mode: str = "allow_once",
        enable_terminal: bool = True,
        enable_filesystem: bool = True,
        stdio_buffer_limit_bytes: int = DEFAULT_STDIO_BUFFER_LIMIT_BYTES,
        auth_method: str | None = None,
        extra_env: dict[str, str] | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(logs_dir=logs_dir, model_name=model_name, **kwargs)
        self.backend_artifact_path = backend_artifact_path
        self.backend_args = backend_args or ""
        self.install_dir = install_dir
        self.session_cwd = session_cwd
        self.permission_mode = permission_mode
        self.enable_terminal = enable_terminal
        self.enable_filesystem = enable_filesystem
        self.stdio_buffer_limit_bytes = stdio_buffer_limit_bytes
        self.auth_method = auth_method
        self.extra_env = extra_env or {}

    def version(self) -> str:
        return "0.1.0"

    @property
    @abstractmethod
    def _install_agent_template_path(self) -> Path:
        pass

    @property
    def _template_variables(self) -> dict[str, Any]:
        return {}

    @property
    @abstractmethod
    def _backend_command(self) -> str:
        pass

    @property
    def _staged_backend_filename(self) -> str:
        return "backend"

    @property
    def _staged_auth_filename(self) -> str:
        return "auth.json"

    @abstractmethod
    def _default_backend_artifact_path(self) -> Path:
        pass

    def _resolved_backend_artifact_path(self) -> Path:
        if self.backend_artifact_path:
            return Path(self.backend_artifact_path)
        return self._default_backend_artifact_path()

    def _resolved_backend_argv(self) -> list[str]:
        return [self._backend_command, *shlex.split(self.backend_args)]

    def _resolved_host_auth_path(self) -> Path | None:
        return None

    def _resolved_runtime_env(self) -> dict[str, str]:
        return dict(self.extra_env)

    async def _upload_additional_inputs(self, environment: BaseEnvironment) -> None:
        del environment

    def _staged_backend_binary_path(self) -> str:
        return str(Path(self.install_dir) / self._staged_backend_filename)

    def _staged_auth_path(self) -> str:
        return str(Path(self.install_dir) / self._staged_auth_filename)

    async def _post_new_session(self, conn: Any, session: Any) -> None:
        del conn, session

    def _render_setup_script(self) -> str:
        if not self._install_agent_template_path.exists():
            raise FileNotFoundError(
                f"ACP setup template not found: {self._install_agent_template_path}"
            )
        env = Environment()
        template = env.from_string(self._install_agent_template_path.read_text())
        return template.render(**self._template_variables)

    async def _run_setup_script(self, environment: BaseEnvironment) -> None:
        script_path = self.logs_dir / "install.sh"
        script_path.write_text(self._render_setup_script(), encoding="utf-8")

        await environment.exec(command=f"mkdir -p {shlex.quote(self.install_dir)} /installed-agent")
        await environment.upload_file(script_path, self.DEFAULT_SETUP_SCRIPT_PATH)

        result = await environment.exec(
            command=f"bash {shlex.quote(self.DEFAULT_SETUP_SCRIPT_PATH)}",
            env={"DEBIAN_FRONTEND": "noninteractive"},
        )

        setup_dir = self.logs_dir / "setup"
        setup_dir.mkdir(parents=True, exist_ok=True)
        (setup_dir / "return-code.txt").write_text(str(result.return_code), encoding="utf-8")
        if result.stdout:
            (setup_dir / "stdout.txt").write_text(result.stdout, encoding="utf-8")
        if result.stderr:
            (setup_dir / "stderr.txt").write_text(result.stderr, encoding="utf-8")

        if result.return_code != 0:
            raise RuntimeError(
                f"ACP container setup failed with exit code {result.return_code}. "
                f"See logs in {setup_dir}"
            )

    async def setup(self, environment: BaseEnvironment) -> None:
        artifact_path = self._resolved_backend_artifact_path()
        await environment.exec(command=f"mkdir -p {shlex.quote(self.install_dir)} /installed-agent")
        await environment.upload_file(artifact_path, self._staged_backend_binary_path())

        host_auth_path = self._resolved_host_auth_path()
        if host_auth_path is not None:
            await environment.upload_file(host_auth_path, self._staged_auth_path())

        await self._upload_additional_inputs(environment)
        await self._run_setup_script(environment)

    def _docker_exec_argv(
        self,
        environment: BaseEnvironment,
        *,
        runtime_env: dict[str, str],
    ) -> list[str]:
        compose_paths = getattr(environment, "_docker_compose_paths", None)
        env_vars = getattr(environment, "_env_vars", None)
        environment_dir = getattr(environment, "environment_dir", None)
        session_id = getattr(environment, "session_id", None)
        if (
            compose_paths is None
            or env_vars is None
            or environment_dir is None
            or session_id is None
        ):
            raise RuntimeError(
                "BaseAcpAgent currently requires Harbor's DockerEnvironment."
            )

        argv = [
            "docker",
            "compose",
            "-p",
            str(session_id).lower().replace(".", "-"),
            "--project-directory",
            str(Path(environment_dir).resolve().absolute()),
        ]
        for path in compose_paths:
            argv.extend(["-f", str(Path(path).resolve().absolute())])
        argv.extend(["exec", "-T", "-w", self.session_cwd])
        for key, value in runtime_env.items():
            argv.extend(["-e", f"{key}={value}"])
        argv.append("main")
        argv.extend(self._resolved_backend_argv())
        return argv

    async def run(
        self,
        instruction: str,
        environment: BaseEnvironment,
        context: AgentContext,
    ) -> None:
        stderr_path = self.logs_dir / "backend-stderr.txt"
        debug_log = self.logs_dir / "container-debug.log"
        runtime_env = self._resolved_runtime_env()
        compose_env_vars = getattr(environment, "_env_vars", None)
        if compose_env_vars is None:
            raise RuntimeError(
                "BaseAcpAgent could not access Harbor Docker compose environment variables."
            )
        subprocess_env = compose_env_vars.to_env_dict(include_os_env=True)
        subprocess_env.update(os.environ)

        bridge_log = self.logs_dir / "bridge.json"
        client = HarborAcpClient(
            environment=environment,
            logs_dir=self.logs_dir,
            logger=self.logger,
            session_cwd=self.session_cwd,
            permission_mode=self.permission_mode,
            enable_terminal=self.enable_terminal,
        )
        conn = None
        proc: asyncio.subprocess.Process | None = None
        stderr_handle = stderr_path.open("w", encoding="utf-8")

        try:
            debug_log.write_text("starting docker compose exec backend\n", encoding="utf-8")
            argv = self._docker_exec_argv(environment, runtime_env=runtime_env)
            proc = await asyncio.create_subprocess_exec(
                *argv,
                env=subprocess_env,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=stderr_handle,
                limit=self.stdio_buffer_limit_bytes,
            )
            if proc.stdin is None or proc.stdout is None:
                raise RuntimeError(
                    "Container-backed ACP subprocess did not expose stdio pipes."
                )

            debug_log.write_text(
                debug_log.read_text(encoding="utf-8")
                + "initializing acp connection over docker exec\n",
                encoding="utf-8",
            )
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
            if self.auth_method is not None:
                debug_log.write_text(
                    debug_log.read_text(encoding="utf-8")
                    + f"authenticating via {self.auth_method}\n",
                    encoding="utf-8",
                )
                await conn.authenticate(self.auth_method)

            debug_log.write_text(
                debug_log.read_text(encoding="utf-8") + "creating session\n",
                encoding="utf-8",
            )
            session = await conn.new_session(cwd=self.session_cwd, mcp_servers=[])
            await self._post_new_session(conn, session)
            debug_log.write_text(
                debug_log.read_text(encoding="utf-8") + "prompting session\n",
                encoding="utf-8",
            )
            prompt_response = await conn.prompt(
                prompt=[text_block(instruction)],
                session_id=session.session_id,
            )
            debug_log.write_text(
                debug_log.read_text(encoding="utf-8") + "prompt completed\n",
                encoding="utf-8",
            )

            if prompt_response.usage is not None:
                usage = prompt_response.usage
                context.n_input_tokens = usage.input_tokens
                context.n_output_tokens = usage.output_tokens
                context.n_cache_tokens = usage.cached_read_tokens

            context.metadata = {
                "backend_command": self._backend_command,
                "backend_args": self._resolved_backend_argv()[1:],
                "backend_artifact_path": str(self._resolved_backend_artifact_path()),
                "backend_location": "container",
                "stop_reason": prompt_response.stop_reason,
                "session_id": session.session_id,
                "agent_info": _jsonable(init_response.agent_info)
                if init_response.agent_info
                else None,
            }

            bridge_log.write_text(
                json.dumps(
                    {
                        "backend_location": "container",
                        "backend_command": self._backend_command,
                        "backend_args": self._resolved_backend_argv()[1:],
                        "backend_artifact_path": str(self._resolved_backend_artifact_path()),
                        "host_auth_path": (
                            str(self._resolved_host_auth_path())
                            if self._resolved_host_auth_path()
                            else None
                        ),
                        "docker_exec_argv": argv,
                        "session_cwd": self.session_cwd,
                        "auth_method": self.auth_method,
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
