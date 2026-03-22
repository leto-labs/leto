from __future__ import annotations

import json
import os
import shlex
from pathlib import Path
from typing import Any

from harbor.agents.installed.base import BaseInstalledAgent, ExecInput
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext
from harbor.models.trial.paths import EnvironmentPaths


class BrainAgent(BaseInstalledAgent):
    SUPPORTS_ATIF: bool = True
    _SETUP_TEMPLATE_PATH = Path(__file__).with_name("install-brain.sh.j2")
    DEFAULT_STAGING_DIR = "/installed-agent/staged"

    @staticmethod
    def name() -> str:
        return "brain"

    def __init__(
        self,
        logs_dir: Path,
        model_name: str | None = None,
        backend_artifact_path: str | None = None,
        install_dir: str = DEFAULT_STAGING_DIR,
        session_cwd: str = "/app",
        provider: str | None = None,
        brain_loop: str = "simple",
        base_url: str | None = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(logs_dir=logs_dir, model_name=model_name, **kwargs)
        self.backend_artifact_path = backend_artifact_path
        self.install_dir = install_dir
        self.session_cwd = session_cwd
        self.provider = provider
        self.brain_loop = brain_loop
        self.base_url = base_url

    def version(self) -> str | None:
        return "0.1.0"

    @property
    def _install_agent_template_path(self) -> Path:
        return self._SETUP_TEMPLATE_PATH

    @property
    def _template_variables(self) -> dict[str, str]:
        return {
            "staged_backend_path": self._staged_backend_path(),
        }

    def _default_backend_artifact_path(self) -> Path:
        repo_root = Path(__file__).resolve().parents[3]
        for candidate in (
            repo_root / "target" / "x86_64-unknown-linux-musl" / "release" / "brain",
            repo_root / "target" / "release" / "brain",
            repo_root / "target" / "debug" / "brain",
        ):
            if candidate.exists():
                return candidate
        raise RuntimeError(
            "Unable to locate a local brain binary. Build "
            "target/x86_64-unknown-linux-musl/release/brain, "
            "target/release/brain, or target/debug/brain first, or pass "
            "backend_artifact_path."
        )

    def _resolved_backend_artifact_path(self) -> Path:
        if self.backend_artifact_path:
            return Path(self.backend_artifact_path)
        return self._default_backend_artifact_path()

    def _staged_backend_path(self) -> str:
        return str(Path(self.install_dir) / "brain")

    async def setup(self, environment: BaseEnvironment) -> None:
        await environment.exec(command=f"mkdir -p {shlex.quote(self.install_dir)} /installed-agent")
        await environment.upload_file(
            self._resolved_backend_artifact_path(),
            self._staged_backend_path(),
        )
        await super().setup(environment)

    def _resolved_provider(self) -> str:
        if self.provider:
            return self.provider
        if self.model_name and "/" in self.model_name:
            provider, _model = self.model_name.split("/", maxsplit=1)
            return provider
        raise RuntimeError(
            "BrainAgent requires provider=<name> when model_name is not provider-qualified."
        )

    def _resolved_model(self) -> str:
        if not self.model_name:
            raise RuntimeError("BrainAgent requires model_name.")
        if "/" in self.model_name:
            _provider, model = self.model_name.split("/", maxsplit=1)
            return model
        return self.model_name

    def create_run_agent_commands(self, instruction: str) -> list[ExecInput]:
        api_key = os.environ.get("HARBOR_API_KEY") or os.environ.get("OPENAI_API_KEY")
        if not api_key:
            raise RuntimeError("BrainAgent requires api_key for direct Harbor runs.")

        base_url = self.base_url or os.environ.get("HARBOR_BASE_URL") or os.environ.get(
            "OPENAI_BASE_URL"
        )
        env = {"BRAIN_API_KEY": api_key}
        if base_url:
            env["BRAIN_BASE_URL"] = base_url

        command = (
            "brain exec "
            f"--cwd {shlex.quote(self.session_cwd)} "
            f"--provider {shlex.quote(self._resolved_provider())} "
            f"--model {shlex.quote(self._resolved_model())} "
            f"--loop {shlex.quote(self.brain_loop)} "
            f"--output-dir {shlex.quote(EnvironmentPaths.agent_dir.as_posix())} "
            '--api-key "$BRAIN_API_KEY" '
        )
        if base_url:
            command += '--base-url "$BRAIN_BASE_URL" '
        command += f"-- {shlex.quote(instruction)}"

        return [ExecInput(command=command, env=env, cwd=self.session_cwd)]

    def populate_context_post_run(self, context: AgentContext) -> None:
        run_path = self.logs_dir / "run.json"
        if not run_path.exists():
            return

        payload = json.loads(run_path.read_text())
        trajectory_path = self.logs_dir / "trajectory.json"
        if not trajectory_path.exists():
            raise RuntimeError("BrainAgent expected trajectory.json to be emitted by brain exec.")
        input_tokens = payload.get("input_tokens")
        output_tokens = payload.get("output_tokens")
        cache_tokens = payload.get("cache_tokens")
        cost_usd = payload.get("cost_usd")

        if isinstance(input_tokens, int):
            context.n_input_tokens = input_tokens
        if isinstance(output_tokens, int):
            context.n_output_tokens = output_tokens
        if isinstance(cache_tokens, int):
            context.n_cache_tokens = cache_tokens
        if isinstance(cost_usd, (int, float)):
            context.cost_usd = float(cost_usd)
