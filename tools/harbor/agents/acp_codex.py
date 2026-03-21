from __future__ import annotations

import glob
import os
import platform
from pathlib import Path
from typing import Any

from .acp_base import BaseAcpAgent


def _host_arch_to_codex_acp_suffix() -> str:
    machine = platform.machine().lower()
    if machine in {"x86_64", "amd64"}:
        return "x64"
    if machine in {"aarch64", "arm64"}:
        return "arm64"
    raise RuntimeError(f"Unsupported host architecture for codex-acp binary: {machine}")


def _detect_codex_acp_binary() -> Path:
    suffix = _host_arch_to_codex_acp_suffix()
    patterns = [
        os.path.expanduser(
            f"~/.local/share/pnpm/global/5/.pnpm/"
            f"@zed-industries+codex-acp-linux-{suffix}@*/node_modules/"
            f"@zed-industries/codex-acp-linux-{suffix}/bin/codex-acp"
        ),
    ]
    for pattern in patterns:
        matches = sorted(glob.glob(pattern))
        if matches:
            return Path(matches[-1])
    raise RuntimeError(
        "Unable to locate a local codex-acp Linux binary. "
        "Install codex-acp on the host first."
    )


def _detect_host_codex_auth_json() -> Path | None:
    path = Path.home() / ".codex" / "auth.json"
    if path.exists():
        return path
    return None


class AcpCodexAgent(BaseAcpAgent):
    CODEX_ACP_COMMAND = "/usr/local/bin/codex-acp"
    DEFAULT_CODEX_HOME = "/logs/agent/codex-home"
    _SETUP_TEMPLATE_PATH = Path(__file__).with_name("install-codex-acp.sh.j2")

    @staticmethod
    def name() -> str:
        return "acp-codex"

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
        codex_home: str = DEFAULT_CODEX_HOME,
        host_codex_auth_path: str | None = None,
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
        self.codex_home = codex_home
        self.host_codex_auth_path = host_codex_auth_path

    @property
    def _install_agent_template_path(self) -> Path:
        return self._SETUP_TEMPLATE_PATH

    @property
    def _backend_command(self) -> str:
        return self.CODEX_ACP_COMMAND

    @property
    def _staged_backend_filename(self) -> str:
        return "codex-acp"

    def _default_backend_artifact_path(self) -> Path:
        return _detect_codex_acp_binary()

    def _resolved_host_auth_path(self) -> Path | None:
        if self.host_codex_auth_path:
            path = Path(self.host_codex_auth_path).expanduser()
            if not path.exists():
                raise RuntimeError(f"Configured host_codex_auth_path does not exist: {path}")
            return path
        return _detect_host_codex_auth_json()

    @property
    def _template_variables(self) -> dict[str, Any]:
        return {
            "version": None,
            "staged_backend_path": self._staged_backend_binary_path(),
            "codex_home": self.codex_home,
            "staged_auth_path": self._staged_auth_path(),
        }

    def _resolved_backend_argv(self) -> list[str]:
        argv = [self.CODEX_ACP_COMMAND, *super()._resolved_backend_argv()[1:]]
        if self.model_name and "model=" not in self.backend_args:
            model = self.model_name.split("/", 1)[-1]
            argv.extend(["-c", f'model="{model}"'])
        return argv

    def _resolved_runtime_env(self) -> dict[str, str]:
        runtime_env = {"CODEX_HOME": self.codex_home, **self.extra_env}
        host_auth_path = self._resolved_host_auth_path()
        if self.auth_method == "openai-api-key":
            if openai_api_key := os.environ.get("OPENAI_API_KEY"):
                runtime_env.setdefault("OPENAI_API_KEY", openai_api_key)
            if "OPENAI_API_KEY" not in runtime_env:
                raise RuntimeError(
                    "OPENAI_API_KEY is required for container-backed codex-acp runs when "
                    "auth_method=openai-api-key."
                )
        elif self.auth_method is None and host_auth_path is None:
            raise RuntimeError(
                "Container-backed codex-acp requires either a host Codex auth file "
                "(~/.codex/auth.json) or auth_method=openai-api-key."
            )
        return runtime_env
