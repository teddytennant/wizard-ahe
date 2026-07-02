#!/bin/bash
# Verifier for the log-parser task. Deletes any pre-existing summary, re-runs
# the agent's script, and compares the produced JSON to the exact expected
# summary (so the script itself must work, not just a hand-written JSON file).
mkdir -p /logs/verifier

reward=0
if [ -f /app/parse_logs.py ]; then
  rm -f /app/summary.json
  if (cd /app && python3 /app/parse_logs.py); then
    if python3 - <<'PY'
import json
import sys

expected = {
    "counts": {"DEBUG": 7, "INFO": 13, "WARNING": 5, "ERROR": 5},
    "first_timestamp": "2026-06-01T07:59:58",
    "last_timestamp": "2026-06-01T08:20:02",
    "top_error": "timeout talking to payments service",
}

try:
    with open("/app/summary.json") as f:
        got = json.load(f)
except Exception as e:
    print("failed to load /app/summary.json:", e)
    sys.exit(1)

if not isinstance(got, dict) or set(got) != set(expected):
    print(f"top-level keys wrong: got {sorted(got) if isinstance(got, dict) else got!r}")
    sys.exit(1)
if got["counts"] != expected["counts"]:
    print(f"FAIL counts = {got['counts']!r}, expected {expected['counts']!r}")
    sys.exit(1)
for key in ("first_timestamp", "last_timestamp", "top_error"):
    if got[key] != expected[key]:
        print(f"FAIL {key} = {got[key]!r}, expected {expected[key]!r}")
        sys.exit(1)

print("summary.json is correct")
PY
    then
      reward=1
    fi
  else
    echo "[verifier] parse_logs.py exited non-zero"
  fi
fi

echo "$reward" > /logs/verifier/reward.txt
echo "[verifier] log-parser -> reward=$reward"
