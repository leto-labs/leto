from __future__ import annotations

from pathlib import Path
from typing import Any

from harbor.environments.base import BaseEnvironment

from .acp_base import BaseAcpAgent


class AcpBrainAgent(BaseAcpAgent):
    BRAIN_ACP_COMMAND = "/usr/local/bin/brain-acp"
    DEFAULT_BRAIN_HOME = "/logs/agent/brain-home"
    _SETUP_TEMPLATE_PATH = Path(__file__).with_name("install-brain-acp.sh.j2")

    @staticmethod
    def name() -> str:
        return "acp-brain"

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
        brain_home: str = DEFAULT_BRAIN_HOME,
        brain_loop: str | None = None,
        host_brain_home_path: str | None = None,
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
        self.brain_home = brain_home
        self.brain_loop = brain_loop
        self.host_brain_home_path = Path(
            host_brain_home_path or (Path.home() / ".brain")
        ).expanduser()

    @property
    def _install_agent_template_path(self) -> Path:
        return self._SETUP_TEMPLATE_PATH

    @property
    def _backend_command(self) -> str:
        return self.BRAIN_ACP_COMMAND

    @property
    def _staged_backend_filename(self) -> str:
        return "brain-acp"

    @property
    def _staged_auth_filename(self) -> str:
        # BaseAcpAgent already knows how to stage one optional extra file. For
        # Brain, that slot is used for ~/.brain/config.toml rather than auth.
        return "brain-config.toml"

    def _staged_credentials_dir(self) -> str:
        return str(Path(self.install_dir) / "brain-credentials")

    def _resolved_host_credentials_dir(self) -> Path:
        path = self.host_brain_home_path / "credentials"
        if not path.is_dir():
            raise RuntimeError(
                "Container-backed brain-acp requires a host credentials directory at "
                f"{path}."
            )
        return path

    def _resolved_host_config_path(self) -> Path | None:
        path = self.host_brain_home_path / "config.toml"
        if path.exists():
            return path
        return None

    def _default_backend_artifact_path(self) -> Path:
        repo_root = Path(__file__).resolve().parents[3]
        for candidate in (
            repo_root / "target" / "debug" / "brain-acp",
            repo_root / "target" / "release" / "brain-acp",
        ):
            if candidate.exists():
                return candidate
        raise RuntimeError(
            "Unable to locate a local brain-acp binary. Build target/debug/brain-acp "
            "or target/release/brain-acp first, or pass backend_artifact_path."
        )

    async def _upload_additional_inputs(self, environment: BaseEnvironment) -> None:
        await environment.upload_dir(
            self._resolved_host_credentials_dir(),
            self._staged_credentials_dir(),
        )
        host_config_path = self._resolved_host_config_path()
        if host_config_path is not None:
            await environment.upload_file(host_config_path, self._staged_auth_path())

    async def _post_new_session(self, conn: Any, session: Any) -> None:
        if self.model_name:
            await conn.set_session_model(
                model_id=self.model_name,
                session_id=session.session_id,
            )
        if self.brain_loop:
            await conn.set_config_option(
                config_id="loop",
                session_id=session.session_id,
                value=self.brain_loop,
            )

    @property
    def _template_variables(self) -> dict[str, Any]:
        return {
            "staged_backend_path": self._staged_backend_binary_path(),
            "brain_home": self.brain_home,
            "staged_credentials_dir": self._staged_credentials_dir(),
            "staged_config_path": self._staged_auth_path(),
        }

    def _resolved_runtime_env(self) -> dict[str, str]:
        return {
            "BRAIN_HOME": self.brain_home,
            **self.extra_env,
        }
