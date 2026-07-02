#!/bin/bash
# Verifier for the fix-failing-test task. Reward 1.0 iff:
#   1. /app/test_stats_util.py is byte-identical to the shipped version (sha256),
#   2. the shipped pytest suite passes,
#   3. median() also passes extra held-out cases (guards against overfitting).
mkdir -p /logs/verifier

expected_sha="b3e1b2bbfd3c9c161b15743f83fce3e02bba9d2bfff2da80327e4f4f68ccddf9"
reward=0

actual_sha=$(sha256sum /app/test_stats_util.py 2>/dev/null | awk '{print $1}')
if [ "$actual_sha" != "$expected_sha" ]; then
  echo "[verifier] test file missing or modified (sha256 mismatch)"
else
  if (cd /app && python3 -m pytest -q test_stats_util.py); then
    if python3 - <<'PY'
import sys
sys.path.insert(0, "/app")
from stats_util import median

held_out = [
    ([1, 2, 3, 4, 5, 6], 3.5),
    ([2, 1], 1.5),
    ([10, -10], 0.0),
    ([5, 5, 5, 5], 5.0),
    ([1.5, 0.5, 2.5], 1.5),
    ([100], 100),
]
for values, want in held_out:
    got = median(values)
    if got != want:
        print(f"FAIL median({values}) = {got}, expected {want}")
        sys.exit(1)

try:
    median([])
except ValueError:
    pass
else:
    print("FAIL median([]) should raise ValueError")
    sys.exit(1)

print("held-out cases passed")
PY
    then
      reward=1
    fi
  fi
fi

echo "$reward" > /logs/verifier/reward.txt
echo "[verifier] fix-failing-test -> reward=$reward"
