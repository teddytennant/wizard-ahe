# wizard-ahe

Grok 4.5 powered [AHE](https://github.com/china-qijizhifeng/agentic-harness-engineering) for [wizard](https://github.com/teddytennant/wizard).

Runs wizard on a Docker task set, reads the transcripts, and has Grok 4.5 rewrite the harness bundle (system prompt, tool descriptions, skills, subagents). Measure again. Keep the wins, roll back the rest. Human review, then bake the bundle back into wizard as the new baseline.

Grok 4.5 is the default brain for both wizard and the evolve agent. Any OpenAI-compatible endpoint still works.

## Attribution

Fork of **Agentic Harness Engineering** by Curry09 (Jiahang Lin) and contributors. Methodology, evolve/debugger agents, and framework are theirs.

- Upstream: https://github.com/china-qijizhifeng/agentic-harness-engineering
- Paper: [arXiv:2604.25850](https://arxiv.org/abs/2604.25850)
- MIT (see [LICENSE](LICENSE))
## How it works

1. `agents/wizard_harness/` is the evolve target (`wizard harness export`).
2. `evolve.py` copies it into a git-tracked workspace; each edit is a commit.
3. The harbor adapter drops the wizard binary + candidate bundle into each task container and runs `wizard -p`.
4. Verifiers score rollouts. Bad edits roll back.
5. Task images come from the Nix `wizard-ahe/task-base` build.

## Install

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/teddytennant/wizard-ahe/main/install.sh)
```

Or `./install.sh` from a checkout. Builds the `ahe` CLI, runs `ahe setup` (pick API, ChatGPT OAuth, or xAI/Grok 4.5), installs wizard, builds the task image.

## Usage

```bash
ahe setup                      # idempotent
ahe run --config wizard-smoke  # wiring gate
ahe run                        # 10 tasks x 5 iterations
ahe run -i 2 --tasks even-sum,csv-report -k 2
ahe run --no-progress --json
ahe run --dry-run
ahe status
```

Full runbook and backends: [docs/WIZARD-AHE.md](docs/WIZARD-AHE.md).

## Layout

```
agents/
  wizard_harness/   evolve target (exported harness bundle)
  wizard_agent/     harbor adapter
  evolve_agent/     AHE meta-agent
configs/            base.yaml + exp-wizard*.yaml
dataset/wizard/     10 agentic coding tasks
dataset/local-sample/  hello-file smoke task
cli/                ahe (Rust)
docs/WIZARD-AHE.md  runbook
scripts/            helpers, OAuth proxies, image build
install.sh          one-liner
evolve.py           evaluate -> analyze -> improve
flake.nix           task-base image + dev shell
```
