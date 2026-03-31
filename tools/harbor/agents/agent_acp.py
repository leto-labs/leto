from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from harbor.environments.base import BaseEnvironment

from .acp_base import BaseAcpAgent


class HarborAcpAgent(BaseAcpAgent):
    AGENT_ACP_COMMAND = "/usr/local/bin/agent-acp"
    DEFAULT_AGENT_HOME = "/logs/agent/agent-home"
    _SETUP_TEMPLATE_PATH = Path(__file__).with_name("install-agent-acp.sh.j2")

    @staticmethod
    def name() -> str:
        return "agent-acp"

    def __init__(
        self,
        logs_dir: Path,
        model_name: str | None = None,
        backend_artifact_path: str | None = None,
        backend_args: str = "",
        install_dir: str = BaseAcpAgent.DEFAULT_STAGING_DIR,
        session_cwd: str = "/app",
        permission_mode: str = "allow_once",
        enable_terminal: bool = True,
        enable_filesystem: bool = True,
        stdio_buffer_limit_bytes: int = BaseAcpAgent.DEFAULT_STDIO_BUFFER_LIMIT_BYTES,
        auth_method: str | None = None,
        agent_home: str = DEFAULT_AGENT_HOME,
        agent_loop: str | None = None,
        provider: str | None = None,
        base_url: str | None = None,
        api_surface: str | None = None,
        host_agent_home_path: str | None = None,
        extra_env: dict[str, str] | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(
            logs_dir=logs_dir,
            model_name=model_name,
            backend_artifact_path=backend_artifact_path,
            backend_args=backend_args,
            install_dir=install_dir,
            session_cwd=session_cwd,
            permission_mode=permission_mode,
            enable_terminal=enable_terminal,
            enable_filesystem=enable_filesystem,
            stdio_buffer_limit_bytes=stdio_buffer_limit_bytes,
            auth_method=auth_method,
            extra_env=extra_env,
            **kwargs,
        )
        self.agent_home = agent_home
        self.agent_loop = agent_loop
        self.provider = provider
        self.base_url = base_url
        self.api_surface = api_surface
        self.host_agent_home_path = Path(
            host_agent_home_path or (Path.home() / ".brain")
        ).expanduser()

    @property
    def _install_agent_template_path(self) -> Path:
        return self._SETUP_TEMPLATE_PATH

    @property
    def _backend_command(self) -> str:
        return self.AGENT_ACP_COMMAND

    @property
    def _staged_backend_filename(self) -> str:
        return "agent-acp"

    @property
    def _staged_auth_filename(self) -> str:
        return "agent-config.toml"

    def _staged_credentials_dir(self) -> str:
        return str(Path(self.install_dir) / "agent-credentials")

    def _resolved_host_credentials_dir(self) -> Path | None:
        path = self.host_agent_home_path / "credentials"
        if path.is_dir():
            return path
        return None

    def _resolved_host_config_path(self) -> Path | None:
        path = self.host_agent_home_path / "config.toml"
        if path.exists():
            return path
        return None

    def _default_backend_artifact_path(self) -> Path:
        repo_root = Path(__file__).resolve().parents[3]
        for candidate in (
            repo_root / "target" / "x86_64-unknown-linux-musl" / "release" / "agent-acp",
            repo_root / "target" / "release" / "agent-acp",
            repo_root / "target" / "debug" / "agent-acp",
        ):
            if candidate.exists():
                return candidate
        raise RuntimeError(
            "Unable to locate a local agent-acp binary. Build "
            "target/x86_64-unknown-linux-musl/release/agent-acp, "
            "target/release/agent-acp, or target/debug/agent-acp first, or pass "
            "backend_artifact_path."
        )

    async def _upload_additional_inputs(self, environment: BaseEnvironment) -> None:
        host_credentials_dir = self._resolved_host_credentials_dir()
        if host_credentials_dir is not None:
            await environment.upload_dir(
                host_credentials_dir,
                self._staged_credentials_dir(),
            )
        host_config_path = self._resolved_host_config_path()
        if host_config_path is not None:
            await environment.upload_file(host_config_path, self._staged_auth_path())

    async def _post_new_session(self, conn: Any, session: Any) -> None:
        if self.model_name:
            model_value = self.model_name.split("/", 1)[-1]
            await conn.set_config_option(
                config_id="model",
                session_id=session.session_id,
                value=model_value,
            )
        if self.agent_loop:
            await conn.set_config_option(
                config_id="loop",
                session_id=session.session_id,
                value=self.agent_loop,
            )

    @property
    def _template_variables(self) -> dict[str, Any]:
        return {
            "staged_backend_path": self._staged_backend_binary_path(),
            "agent_home": self.agent_home,
            "staged_credentials_dir": self._staged_credentials_dir(),
            "staged_config_path": self._staged_auth_path(),
        }

    def _resolved_provider(self) -> str | None:
        if self.provider:
            return self.provider
        if self.model_name and "/" in self.model_name:
            provider, _model = self.model_name.split("/", maxsplit=1)
            return provider
        return None

    def _resolved_runtime_env(self) -> dict[str, str]:
        runtime_env = {
            "AGENT_HOME": self.agent_home,
            **self.extra_env,
        }
        if self.model_name:
            runtime_env["AGENT_MODEL"] = self.model_name.split("/", 1)[-1]
        if provider := self._resolved_provider():
            runtime_env["AGENT_PROVIDER"] = provider
        if self.base_url:
            runtime_env["AGENT_BASE_URL"] = self.base_url
        if self.api_surface:
            runtime_env["AGENT_API_SURFACE"] = self.api_surface
        if self.agent_loop:
            runtime_env["AGENT_DEFAULT_LOOP"] = self.agent_loop

        if self.auth_method == "openai-api-key":
            api_key = os.environ.get("HARBOR_API_KEY") or os.environ.get("OPENAI_API_KEY")
            if not api_key:
                raise RuntimeError(
                    "OPENAI_API_KEY or HARBOR_API_KEY is required for Harbor agent-acp runs "
                    "when auth_method=openai-api-key."
                )
            runtime_env["AGENT_API_KEY"] = api_key
        return runtime_env
