# Running the AHE evolve loop on wizard

AHE used as an external lab tool to improve [`wizard`](https://github.com/teddytennant/wizard):
it runs wizard over a task set in Docker, analyzes failures, has a meta-model
rewrite wizard's **harness bundle** — system prompt, tool descriptions, skills,
subagents — and re-measures, producing a before/after pass-rate. Wizard's native
Rust runtime is untouched; only the externalized bundle evolves.

The loop is provider-generic: any **OpenAI-compatible endpoint** works, configured
entirely from `.env` (`LLM_BASE_URL`, `LLM_API_KEY`, `LLM_MODEL`). Local
llama-server, a cloud API, or the xAI OAuth proxy (appendix) are all just
different values for those three variables.

## The harness bundle

The evolve target `agents/wizard_harness/` is a full bundle produced by
`wizard harness export` (wizard branch `feat/harness-bundle`):

```
agents/wizard_harness/
  system_prompt.md            # base personality prompt (sovereign mode)
  tool_descriptions/<tool>.md # description advertised to the model per native tool
  skills/<name>/SKILL.md      # prompt-injected skills
  subagents/<name>.toml       # spawnable subagent definitions
  HARNESS.md                  # generated guide (rides into the workspace,
                              # orients the evolve agent)
```

AHE copies the whole directory into `experiments/<run>/workspace/` (git-tracked,
one commit per evolve edit); the adapter tars the current candidate into each
task container at `/root/.wizard/harness` and runs wizard with
`WIZARD_HARNESS_DIR` pointing at it. Missing/empty files fall back to wizard's
compiled defaults, so destructive edits degrade gracefully instead of bricking
the run.

## What's wired

- **wizard** (`feat/harness-bundle`): loads a full harness bundle from
  `--harness-dir` / `$WIZARD_HARNESS_DIR`; `wizard harness export <dir>` dumps
  the compiled defaults as a bundle. Sentinel-verified end-to-end (bundle
  contents reach the LLM request).
- **harbor adapter** `agents/wizard_agent/adapter.py` (+ `install-wizard.sh.j2`):
  uploads the wizard binary + the current candidate bundle (tarball, `.git`
  excluded) into each task container, writes `~/.wizard/config.toml` pointing at
  `WIZARD_LLM_BASE_URL`, runs `wizard -p` with `WIZARD_HARNESS_DIR` set.
  Selected by import path.
- **evolve.py**: emits `--agent-import-path` when `harbor.agent_import_path` is
  set, so the custom adapter is usable without touching harbor's `AgentName` enum.
- **configs** `configs/experiments/exp-wizard.yaml` (k=2, 5 iters, debugger off,
  transcript-fed) and `exp-wizard-smoke.yaml` (hello-file, k=1, 1 iter) — both
  read `${LLM_BASE_URL}/${LLM_API_KEY}/${LLM_MODEL}` from `.env`.
- **dataset** `dataset/wizard/` — 10 verifiable agentic coding tasks (see its
  README) + `dataset/local-sample/hello-file` as the smoke gate.
- **scripts** `scripts/evolve-wizard.sh` (run the loop); optional backends in
  the appendix.

## Run it

```bash
# 0. Build wizard from the harness-bundle branch and seed/refresh the bundle
cd wizard && git checkout feat/harness-bundle && cargo build --release
export WIZARD_BINARY=/abs/path/to/wizard/target/release/wizard
./target/release/wizard harness export /abs/path/to/wizard-ahe/agents/wizard_harness

# 1. Point AHE at any OpenAI-compatible endpoint
cd wizard-ahe
cp .env.example .env   # then set:
#   LLM_BASE_URL=...       e.g. http://localhost:8080/v1, https://api.x.ai/v1, ...
#   LLM_API_KEY=...        any non-empty string for keyless local servers
#   LLM_MODEL=...          model name at that endpoint
uv sync

# 2. In-container endpoint for the agent-under-test (task containers can't
#    always see "localhost" of the host):
export WIZARD_LLM_BASE_URL=http://host.docker.internal:8080/v1   # or your API URL

# 3. Smoke gate — prove the wiring on the trivial task before the real run
uv run python evolve.py --config configs/experiments/exp-wizard-smoke.yaml

# 4. Full run
./scripts/evolve-wizard.sh
# baseline vs final:        experiments/<run>/iteration_scores.yaml
# evolved bundle + history: experiments/<run>/workspace/  (git log -p)
```

Container → host networking (Linux): when the endpoint runs on the host, the
adapter defaults to `http://host.docker.internal:8080/v1`; harbor must launch
task containers with `--add-host=host.docker.internal:host-gateway` (or
`--network host` + `WIZARD_LLM_BASE_URL=http://localhost:8080/v1`). A remote
API endpoint needs no special networking, just `allow_internet = true` in the
task (already set).

## Merge-back: making the loop recursive

One cycle, always human-gated:

1. Run the loop; the best bundle sits in `experiments/<run>/workspace/`
   (per-generation snapshots under `runs/iteration_NNN/`; regressed predictions
   are rolled back automatically).
2. Review every evolve edit: `git -C experiments/<run>/workspace log -p`, and
   diff the final bundle against `agents/wizard_harness/`.
3. Bake accepted changes into wizard on a branch:
   - `system_prompt.md` → the prompt constants in `src/agent/prompts.rs`
   - `tool_descriptions/*.md` → the description strings in `src/tools/*.rs`
   - `skills/**` → wizard's `skills/`
   - `subagents/*.toml` → wizard's `loadout/subagents/`
   Gate on `cargo test` + a `wizard bench` before/after replay + PR review.
4. Rebuild wizard, re-run `wizard harness export agents/wizard_harness/` here,
   commit. The next AHE round starts from the improved baseline — that's the
   recursion. One cycle = one wizard PR + one seed commit here.

## Open items (validated by the next gate run)

1. **`code_agent_patch` vs markdown prompt.** Neutralized to `{}` in the wizard
   configs; `evolve.py:apply_code_agent_patch` already skips non-YAML configs.
   Re-confirm on the first full-bundle run.
2. **Container networking flag.** Confirm harbor's docker env applies
   `--add-host=...:host-gateway` when using a host-local endpoint.
3. **Binary upload path.** The adapter uploads `$WIZARD_BINARY` via
   `environment.upload_file`; confirm glibc compatibility with task base images
   (build static/musl if a task image is Alpine).
4. **Transcript → evolve evidence.** With the debugger off, confirm the evolve
   agent receives wizard's transcript (`/logs/agent/wizard.txt` →
   `trajectory.json`) as failure evidence; wire it into the evolution query if not.

## Cost / scale

10 tasks × k=2 × 5 iters = 100 rollouts + ~5 evolve passes. Cost depends
entirely on the endpoint: ≈ $0 on a self-hosted GPU; on an API, the evolve
passes dominate (long transcript-fed prompts). Optional: point only
`evolve_agent.llm_config` at a stronger API model while the high-volume
agent-under-test stays on a cheap/local endpoint.

---

## Appendix: optional endpoint backends

### Local llama-server (self-hosted GPU)

`./scripts/serve-qwen.sh` serves a Qwen GGUF on `http://0.0.0.0:8080/v1`
(model name `qwen3.6-27b`). Set `LLM_BASE_URL=http://localhost:8080/v1`,
`LLM_API_KEY=sk-noauth-local`, `LLM_MODEL=qwen3.6-27b`. Marginal cost ≈ $0.

### Grok via xAI OAuth subscription (no API key)

wizard's `wizard --login xai` stores a Bearer token (`~/.wizard/xai_oauth.json`)
for the OpenAI-compatible API at `https://api.x.ai/v1`.
`scripts/xai-oauth-proxy.py` re-serves that session as a local
OpenAI-compatible endpoint with auto-refresh; `scripts/run-wizard-xai-loop.sh`
is a turnkey wrapper. Set `LLM_BASE_URL=http://localhost:8088/v1`,
`LLM_API_KEY=oauth-via-proxy`, `LLM_MODEL=grok-4.3`, and
`WIZARD_LLM_BASE_URL=http://172.17.0.1:8088/v1` (docker bridge IP) for the
containers.

Caveats: OAuth API access is gated to certain SuperGrok plans (403 if yours
lacks it — fall back to `XAI_API_KEY`); it's a personal subscription, so keep
`n_concurrent` low; binding the proxy on `0.0.0.0` exposes a token-injecting
endpoint on your LAN (prefer the docker bridge address or `HOST=127.0.0.1` +
`--network host`).
