"""Harbor adapter for the `wizard` CLI (https://github.com/teddytennant/wizard).

Lets AHE's evolve loop use wizard's *native* agent as the agent-under-test. The
loop evolves wizard's externalized *harness bundle* — the directory wizard loads
via `$WIZARD_HARNESS_DIR` (wizard's `feat/harness-bundle`): `system_prompt.md`,
`tool_descriptions/<tool>.md`, `skills/<name>/SKILL.md`, `subagents/<name>.toml`.
This adapter ships the current candidate bundle into each task container and runs
`wizard -p "<instruction>"` headlessly. Harbor's own verifier (`tests/test.sh`)
scores the result — the adapter does not score.

Selected via import path (harbor `--agent-import-path`), so no edit to harbor's
closed `AgentName` enum is needed:

    harbor:
      agent_import_path: "agents.wizard_agent.adapter:WizardAgent"

Wiring contract (host env, read at run time):
  WIZARD_BINARY        host path to the release `wizard` binary to upload
                       (e.g. /home/gradient/projects/ai/wizard/target/release/wizard).
                       Must be built from a branch with harness-bundle support
                       (`wizard harness export` exists).
  WIZARD_LLM_BASE_URL  OpenAI-compatible base_url the in-container wizard talks to;
                       defaults to http://host.docker.internal:8080/v1 (an
                       OpenAI-compatible server on the host; container must be
                       launched with --add-host=host.docker.internal:host-gateway,
                       or use --network host and http://localhost:8080/v1).

The evolved bundle arrives via the `config_path` kwarg (harbor
`--ak config_path=<workspace>/system_prompt.md`, set by evolve.py from
`source_config_dir` + `agent_config_filename`): the bundle root is that file's
parent directory — i.e. the whole evolve workspace is the bundle. The
workspace's `.git` bookkeeping is excluded from the upload.
"""

from __future__ import annotations

import json
import os
import shlex
import tarfile
import tempfile
from pathlib import Path

from harbor.agents.installed.base import BaseInstalledAgent, ExecInput
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext

# Where wizard reads its config and the evolved harness inside the container.
CONTAINER_WIZARD_BIN = "/usr/local/bin/wizard"
CONTAINER_WIZARD_DIR = "/root/.wizard"
CONTAINER_HARNESS_DIR = "/root/.wizard/harness"
DEFAULT_BASE_URL = "http://host.docker.internal:8080/v1"
EVAL_API_KEY_ENV = "WIZARD_EVAL_API_KEY"


class WizardAgent(BaseInstalledAgent):
    """Run the wizard CLI headlessly as a harbor installed agent."""

    def __init__(self, config_path: str | None = None, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        # The evolved system_prompt.md the loop is currently testing (host path).
        # Its parent directory is the full harness bundle.
        self._prompt_source = Path(config_path) if config_path else None
        self._bundle_shipped = False

    @staticmethod
    def name() -> str:
        return "wizard"

    @property
    def _install_agent_template_path(self) -> Path:
        return Path(__file__).parent / "install-wizard.sh.j2"

    async def setup(self, environment: BaseEnvironment) -> None:
        """Upload the wizard binary + evolved harness bundle, then run the install template."""
        binary = os.environ.get("WIZARD_BINARY")
        if not binary or not Path(binary).is_file():
            raise FileNotFoundError(
                "Set WIZARD_BINARY to the host path of the release `wizard` binary "
                f"(got {binary!r}). Build it with `cargo build --release` in the "
                "wizard repo."
            )

        await environment.exec(command=f"mkdir -p {CONTAINER_WIZARD_DIR}")
        await environment.upload_file(
            source_path=Path(binary), target_path=CONTAINER_WIZARD_BIN
        )

        # Ship the whole candidate bundle (system prompt + tool descriptions +
        # skills + subagents) as one tarball; wizard activates it via
        # $WIZARD_HARNESS_DIR at run time. No bundle → wizard uses its baked
        # defaults, which is the correct iteration-0 baseline.
        bundle_root = (
            self._prompt_source.parent
            if self._prompt_source and self._prompt_source.parent.is_dir()
            else None
        )
        if bundle_root:
            tar_path = self._pack_bundle(bundle_root)
            try:
                await environment.upload_file(
                    source_path=tar_path,
                    target_path=f"{CONTAINER_WIZARD_DIR}/harness.tar",
                )
            finally:
                tar_path.unlink(missing_ok=True)
            await environment.exec(
                command=f"mkdir -p {CONTAINER_HARNESS_DIR} && "
                f"tar -xf {CONTAINER_WIZARD_DIR}/harness.tar -C {CONTAINER_HARNESS_DIR} && "
                f"rm {CONTAINER_WIZARD_DIR}/harness.tar"
            )
            self._bundle_shipped = True

        # install-wizard.sh.j2: chmod + `wizard --version` verify.
        await super().setup(environment)

    @staticmethod
    def _pack_bundle(bundle_root: Path) -> Path:
        """Tar the bundle directory (excluding workspace git bookkeeping)."""

        def keep(info: tarfile.TarInfo) -> tarfile.TarInfo | None:
            parts = Path(info.name).parts
            return None if ".git" in parts else info

        with tempfile.NamedTemporaryFile(
            suffix=".tar", prefix="wizard-harness-", delete=False
        ) as tmp:
            tar_path = Path(tmp.name)
        with tarfile.open(tar_path, "w") as tar:
            tar.add(bundle_root, arcname=".", filter=keep)
        return tar_path

    def _config_toml(self) -> str:
        base_url = os.environ.get("WIZARD_LLM_BASE_URL", DEFAULT_BASE_URL)
        model = self.model_name or os.environ.get("WIZARD_MODEL", "")
        # OpenAI-compatible provider → the llama-server on the GPU host. The
        # api_key is read from EVAL_API_KEY_ENV at runtime; llama-server ignores
        # it but wizard's openai provider wants a non-empty value.
        return (
            "mode = \"sovereign\"\n"
            "auto_approve = true\n"
            "max_steps = 50\n"
            "active_provider = \"eval\"\n\n"
            "[[providers]]\n"
            "name = \"eval\"\n"
            "kind = \"openai\"\n"
            f"base_url = \"{base_url}\"\n"
            f"model = \"{model}\"\n"
            f"api_key_env = \"{EVAL_API_KEY_ENV}\"\n"
        )

    def create_run_agent_commands(self, instruction: str) -> list[ExecInput]:
        cfg = shlex.quote(self._config_toml())
        prompt = shlex.quote(instruction)
        env = {EVAL_API_KEY_ENV: os.environ.get(EVAL_API_KEY_ENV, "sk-noauth-local")}
        if self._bundle_shipped:
            env["WIZARD_HARNESS_DIR"] = CONTAINER_HARNESS_DIR
        return [
            ExecInput(
                command=f"mkdir -p {CONTAINER_WIZARD_DIR} && "
                f"printf '%s' {cfg} > {CONTAINER_WIZARD_DIR}/config.toml",
            ),
            ExecInput(
                # Headless run; tee transcript so populate_context can read it and
                # so it survives as failure evidence for the evolve agent.
                command=f"{CONTAINER_WIZARD_BIN} -p {prompt} 2>&1 | tee /logs/agent/wizard.txt",
                env=env,
            ),
        ]

    def populate_context_post_run(self, context: AgentContext) -> None:
        """Best-effort: surface wizard's JSONL session as the trajectory/evidence.

        wizard writes sessions to ~/.wizard/sessions/*.jsonl. Token accounting from
        a local llama-server is not cost-bearing, so we leave cost/token counts at
        their defaults and just expose the transcript for the transcript-fed evolve
        agent. Parsing the JSONL into ATIF is a follow-up (see docs/WIZARD-AHE.md).
        """
        sessions = self.logs_dir / "agent" / "wizard.txt"
        if not sessions.exists():
            return
        try:
            transcript = sessions.read_text(encoding="utf-8", errors="replace")
        except OSError:
            return
        out = self.logs_dir / "trajectory.json"
        try:
            out.write_text(
                json.dumps({"agent": "wizard", "transcript": transcript}, indent=2),
                encoding="utf-8",
            )
        except OSError:
            pass
