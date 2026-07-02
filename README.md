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

## Install (one line)

```bash
bash <(gh api -H "Accept: application/vnd.github.raw" /repos/teddytennant/wizard-ahe/contents/install.sh)
```

(The repo is private, so the one-liner rides an authenticated `gh`; from an
existing checkout just run `./install.sh`.) The installer builds the **`ahe`**
CLI, puts it on your PATH (`~/.local/bin`), and hands off to `ahe setup`,
which asks for the **API base URL, model name, and API key**, probes the
endpoint, installs/builds wizard (static musl), builds the Nix task image,
and syncs the python env — then prints a doctor table.

## Usage

```bash
ahe setup                      # re-run any time; idempotent (flags skip prompts)
ahe run --config wizard-smoke  # wiring gate (1 trivial task)
ahe run                        # full loop: 10 tasks × 5 iterations
ahe run -i 2 --tasks even-sum,csv-report -k 2   # custom slice
ahe run --no-progress --json   # log-friendly output, JSON scores
ahe run --dry-run              # show the effective config without running
ahe status                     # latest experiment + live-run check
```

`ahe run` shows live indicatif progress — iteration bar, per-rollout bar with
a running pass count, and a phase spinner (evaluating / analyzing / evolving)
— then a scores table and the evolved-bundle path.

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
cli/                the `ahe` CLI (Rust: clap + dialoguer + indicatif)
docs/WIZARD-AHE.md  runbook: setup, loop, merge-back, backends
scripts/            evolve-wizard.sh, build-task-image.sh, xai-oauth-proxy.py, ...
install.sh          one-line installer: builds `ahe`, adds to PATH, runs setup
evolve.py           the evaluate → analyze → improve loop (upstream AHE)
flake.nix           Nix-packaged task-base container image + dev shell
```
