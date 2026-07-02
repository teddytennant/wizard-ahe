#!/bin/bash
# Verifier for the cli-wordfreq task. Exercises /app/wordfreq.py across several
# invocations (defaults, --top, --min-len, empty file, missing file) and checks
# stdout and exit codes exactly. Reward -> /logs/verifier/reward.txt.
mkdir -p /logs/verifier

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/sample.txt" <<'EOF'
The cat sat on the mat.
The cat ran!
CATS scatter.
EOF

printf 'abc123def ghi-jkl\n' > "$work/mixed.txt"
: > "$work/empty.txt"

fail() {
  echo "[verifier] FAIL: $1"
  echo "0" > /logs/verifier/reward.txt
  echo "[verifier] cli-wordfreq -> reward=0"
  exit 0
}

[ -f /app/wordfreq.py ] || fail "/app/wordfreq.py missing"

check() {
  local desc="$1" expected="$2"
  shift 2
  local out
  out=$(python3 /app/wordfreq.py "$@" 2>"$work/stderr.txt")
  local status=$?
  [ "$status" -eq 0 ] || fail "$desc: exit status $status (expected 0); stderr: $(cat "$work/stderr.txt")"
  if [ "$out" != "$expected" ]; then
    fail "$desc: got $(printf '%q' "$out"), expected $(printf '%q' "$expected")"
  fi
}

# 1. Defaults: 8 distinct words (< top 10), count desc then alphabetical.
check "defaults" 'the 3
cat 2
cats 1
mat 1
on 1
ran 1
sat 1
scatter 1' "$work/sample.txt"

# 2. --top truncates.
check "--top 2" 'the 3
cat 2' "$work/sample.txt" --top 2

# 3. --min-len filters short words ("on" dropped).
check "--min-len 3" 'the 3
cat 2
cats 1
mat 1
ran 1
sat 1
scatter 1' "$work/sample.txt" --min-len 3

# 4. Both flags together.
check "--top 3 --min-len 4" 'cats 1
scatter 1' "$work/sample.txt" --top 3 --min-len 4

# 5. Digits and punctuation are separators.
check "separators" 'abc 1
def 1
ghi 1
jkl 1' "$work/mixed.txt"

# 6. Empty file prints nothing, exits 0.
check "empty file" '' "$work/empty.txt"

# 7. Missing file: non-zero exit, nothing on stdout.
out=$(python3 /app/wordfreq.py "$work/does-not-exist.txt" 2>/dev/null)
status=$?
[ "$status" -ne 0 ] || fail "missing file: expected non-zero exit status"
[ -z "$out" ] || fail "missing file: expected empty stdout, got $(printf '%q' "$out")"

echo "1" > /logs/verifier/reward.txt
echo "[verifier] cli-wordfreq -> reward=1"
