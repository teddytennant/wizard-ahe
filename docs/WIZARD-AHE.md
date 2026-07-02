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

The easy path is the CLI — `./install.sh` (or the one-liner in the README)
builds `ahe`, adds it to PATH, and walks you through endpoint/model/key,
the wizard build, and the task image:

```bash
ahe setup
ahe run --config wizard-smoke   # wiring gate
ahe run -i 2 --tasks even-sum,csv-report,state-machine   # slice
ahe run                         # full loop with live progress
```

Everything below is the manual equivalent (what `ahe` does under the hood):

```bash
# 0. Build wizard from the harness-bundle branch — STATIC MUSL, so the binary
#    runs inside any task container (Debian- or Nix-based; a host-linked build
#    fails with "required file not found"). On NixOS:
cd wizard && git checkout feat/harness-bundle
nix-shell -p rustup musl --run '
  CC_x86_64_unknown_linux_musl=musl-gcc \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
  RUSTFLAGS="-C target-feature=+crt-static -C link-self-contained=yes" \
  cargo build --release --target x86_64-unknown-linux-musl'
# (link with rust-lld, NOT nix musl-gcc — the latter emits a segfaulting
#  binary with -static and silently dynamic output without it)
export WIZARD_BINARY=/abs/path/to/wizard/target/x86_64-unknown-linux-musl/release/wizard
cargo build --release   # host build, used only for the export below
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

# 3. Build the Nix-packaged task base image (dataset Dockerfiles start FROM it)
./scripts/build-task-image.sh

# 4. Smoke gate — prove the wiring on the trivial task before the real run
uv run python evolve.py --config configs/experiments/exp-wizard-smoke.yaml

# 5. Full run
./scripts/evolve-wizard.sh
# baseline vs final:        experiments/<run>/iteration_scores.yaml
# evolved bundle + history: experiments/<run>/workspace/  (git log -p)
```

Container → host networking (Linux): when the endpoint runs on the host, the
adapter defaults to `http://host.docker.internal:8080/v1`; harbor must launch
task containers with `--add-host=host.docker.internal:host-gateway` (or use
the docker bridge IP, e.g. `WIZARD_LLM_BASE_URL=http://172.17.0.1:8088/v1`).
A remote API endpoint needs no special networking. Either way every task needs
`allow_internet = true` — with it false the container gets NO network and
wizard's LLM health check dies with "Network unreachable".

**NixOS host firewall**: container→host traffic hits the `nixos-fw` input
chain, so the endpoint port must be opened on the bridge or rollouts time out:

```nix
networking.firewall.interfaces."docker0".allowedTCPPorts = [ 8088 ];
```

(one-off equivalent: `sudo iptables -I nixos-fw 1 -i docker0 -p tcp --dport
8088 -j ACCEPT` — not persistent across reboots/rebuilds).

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

1. ~~**`code_agent_patch` vs markdown prompt.**~~ Resolved: neutralized to `{}`
   in the wizard configs and `evolve.py:apply_code_agent_patch` skips non-YAML
   configs; a full-bundle smoke run completed the whole loop (workspace git,
   scoring, evolve pass, iteration_scores) without corrupting the bundle.

   Also NixOS-specific: create the venv with a **uv-managed Python**
   (`uv venv --python-preference only-managed`) — with a nix-store Python,
   harbor's `tokenizers` wheel fails on `libstdc++.so.6` (nix-ld only wraps
   uv-managed interpreters).
2. **Container networking flag.** Confirm harbor's docker env applies
   `--add-host=...:host-gateway` when using a host-local endpoint.
3. ~~**Binary upload path.**~~ Resolved: `$WIZARD_BINARY` must be the static
   musl build (see step 0) — a host-linked binary's ELF interpreter doesn't
   exist inside the containers (bit immediately on NixOS).
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
OpenAI-compatible endpoint with auto-refresh (`PORT=8088 python3
scripts/xai-oauth-proxy.py`). Then it's just .env values for the normal
configs: `LLM_BASE_URL=http://localhost:8088/v1`,
`LLM_API_KEY=oauth-via-proxy`, `LLM_MODEL=grok-4.3`, and
`WIZARD_LLM_BASE_URL=http://172.17.0.1:8088/v1` (docker bridge IP) for the
containers.

Caveats: OAuth API access is gated to certain SuperGrok plans (403 if yours
lacks it — fall back to `XAI_API_KEY`); it's a personal subscription, so keep
`n_concurrent` low; binding the proxy on `0.0.0.0` exposes a token-injecting
endpoint on your LAN (prefer the docker bridge address or `HOST=127.0.0.1` +
`--network host`).
