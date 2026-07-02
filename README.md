# wizard-ahe

Private AHE lab that recursively improves [wizard](https://github.com/teddytennant/wizard)'s
harness. Each round runs wizard over a Docker task set, analyzes its transcripts,
has a meta-agent rewrite wizard's externalized *harness bundle* — system prompt,
tool descriptions, skills, subagents — and re-measures. Regressions roll back
automatically; winning bundles are human-reviewed and baked back into wizard as
new compiled defaults, so the next round starts from an improved baseline.

## Attribution

This repository is a derivative of **Agentic Harness Engineering** by **Curry09 (Jiahang Lin)** and contributors — all credit for the AHE methodology, the evolve/debugger agents, and the original framework goes to them:

- **Upstream repository:** https://github.com/china-qijizhifeng/agentic-harness-engineering
- **Paper:** [Agentic Harness Engineering: Observability-Driven Automatic Evolution of Coding-Agent Harnesses](https://arxiv.org/abs/2604.25850) (arXiv:2604.25850)
- **Built on:** [NexAU](https://github.com/nex-agi/NexAU) by Nex-AGI
- **License:** MIT (upstream copyright retained in [LICENSE](LICENSE))

## How it works

- `agents/wizard_harness/` is the evolve target: a bundle produced by
  `wizard harness export` (`system_prompt.md`, `tool_descriptions/<tool>.md`,
  `skills/<name>/SKILL.md`, `subagents/<name>.toml`, `HARNESS.md`).
- `evolve.py` copies the bundle into a git-tracked workspace; each evolve edit
  is a commit with evidence, root cause, fix, and predicted impact.
- `agents/wizard_agent/adapter.py` (harbor adapter) ships the wizard binary and
  the current candidate bundle into every task container
  (`/root/.wizard/harness`) and runs `wizard -p "<instruction>"` with
  `WIZARD_HARNESS_DIR` set. Missing/empty bundle files fall back to wizard's
  compiled defaults, so broken edits degrade instead of bricking a run.
- Harbor's per-task verifiers score rollouts; flipped tasks falsify the evolve
  agent's predictions and regressed edits are rolled back.
- Task containers are Nix-packaged: `flake.nix` builds the
  `wizard-ahe/task-base` image (`scripts/build-task-image.sh`), and every
  `dataset/wizard/*/environment/Dockerfile` starts from it.

## Quick start

```bash
# endpoint: anything OpenAI-compatible, via .env
#   LLM_BASE_URL=...  LLM_API_KEY=...  LLM_MODEL=...
export WIZARD_BINARY=/path/to/wizard/target/release/wizard   # feat/harness-bundle build
export WIZARD_LLM_BASE_URL=http://host.docker.internal:8080/v1  # containers' view

./scripts/build-task-image.sh                                # Nix task base image
uv sync
uv run python evolve.py --config configs/experiments/exp-wizard-smoke.yaml  # wiring gate
./scripts/evolve-wizard.sh                                   # full run
```

Details, the merge-back procedure, and optional endpoint backends (local
llama-server, xAI OAuth proxy): [docs/WIZARD-AHE.md](docs/WIZARD-AHE.md).

## Layout

```
agents/
  wizard_harness/   evolve target: wizard's exported harness bundle
  wizard_agent/     harbor adapter (binary + bundle into task containers)
  evolve_agent/     AHE's meta-agent (prompts, tools, middleware)
configs/            base.yaml + experiments/exp-wizard{,-smoke}.yaml
dataset/
  wizard/           10 verifiable agentic coding tasks (harbor format)
  local-sample/     trivial hello-file smoke task
docs/WIZARD-AHE.md  runbook: setup, loop, merge-back, backends
scripts/            evolve-wizard.sh, build-task-image.sh, xai-oauth-proxy.py, ...
evolve.py           the evaluate → analyze → improve loop (upstream AHE)
flake.nix           Nix-packaged task-base container image + dev shell
```
