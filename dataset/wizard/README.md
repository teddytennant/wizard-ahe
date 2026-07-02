# wizard evolve task set

Harbor tasks the wizard evolve loop measures pass-rate on. The loop only improves
what you measure, so these should mirror **wizard's real failure modes**, not toy
problems. Target ~15 tasks sized so wizard currently lands around 50–70% — enough
signal for the evolve agent to work with.

## Task format (one dir per task)

```
<task-name>/
  task.toml                 # metadata, timeouts, resources, allow_internet
  instruction.md            # the prompt handed to wizard
  environment/Dockerfile    # the container the agent works in (WORKDIR /app)
  tests/test.sh             # verifier: writes 0/1 to /logs/verifier/reward.txt
  solution/solve.sh         # reference solution (oracle-checks solvability)
```

`even-sum/` is a complete worked example — copy it as a starting point.

## Tasks

| Task | What it measures |
|------|------------------|
| `even-sum` | Worked example: implement a small function (`even_sum`) from a spec. |
| `csv-report` | Read provided data (`data/sales.csv`) and write exact computed output (`report.json` with per-category totals). |
| `fix-failing-test` | Run pytest, localize a bug in the module under test, fix it without touching the test file (checksum-enforced). |
| `cli-wordfreq` | Build a CLI (`wordfreq.py`) with flags, exact output format, and correct exit codes across many invocations. |
| `json-merge` | Implement `deep_merge(a, b)` with precise dict/list/scalar semantics, no input mutation, no shared structure. |
| `refactor-rename` | Multi-file rename across a package (`textkit/`) with identical behavior and zero leftover references. |
| `makefile-build` | Author a Makefile (`build`/`test`/`clean`) that bundles `src/*.txt` sorted, detects stale/corrupt output, and stays glob-based. |
| `log-parser` | Parse a messy log (unordered timestamps, garbage lines) into an exact JSON summary via a rerunnable script. |
| `state-machine` | Implement a `Turnstile` class: transitions, event log, invalid-event `ValueError`s, per-instance state isolation. |
| `git-history` | Drive `git` to build an exact 3-commit history (messages, author/committer, per-commit trees, clean worktree). |

## Required for wizard tasks

- **`allow_internet = true`** in `task.toml`. The in-container wizard must reach
  the LLM served on the host (`host.docker.internal:8080`); with egress blocked
  it cannot call the model and every rollout fails for the wrong reason.
- **Deterministic verifier.** `tests/test.sh` must write exactly `0` or `1` to
  `/logs/verifier/reward.txt`. Prefer exact behavioral checks over string matching.
- **Fast.** Keep each task runnable in seconds–low minutes on local Docker; the
  loop runs every task `k` times across every iteration.
- **Solvable.** `solution/solve.sh` must make the verifier pass (the oracle run
  proves the task is achievable before you spend a loop on it).

## Sourcing ideas (wizard failure modes)

Pull from real wizard sessions / your repos: multi-file edits, running and fixing
a failing test, respecting an existing code style, using `git`, following an
AGENTS.md/CLAUDE.md instruction, recovering from a tool error, etc. Each task
should isolate one capability so a pass-rate change is attributable.
