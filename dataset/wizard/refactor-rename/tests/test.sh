#!/bin/bash
# Verifier for the refactor-rename task. Reward 1.0 iff the old name is fully
# gone from the package source, the new name works with identical behavior,
# and no module exposes the old attribute.
mkdir -p /logs/verifier

reward=0

# Only scan source files: the agent may have created __pycache__/*.pyc that
# legitimately still contain old bytecode.
if grep -rn --include='*.py' "do_the_thing" /app/textkit; then
  echo "[verifier] old name 'do_the_thing' still present in package source"
else
  if python3 - <<'PY'
import sys

sys.path.insert(0, "/app")
try:
    import textkit
    import textkit.core
    import textkit.report
    from textkit import normalize_spaces, summarize
except Exception as e:
    print("import failed:", e)
    sys.exit(1)

for module in (textkit, textkit.core, textkit.report):
    if hasattr(module, "do_the_thing"):
        print(f"FAIL {module.__name__} still exposes do_the_thing")
        sys.exit(1)

if not hasattr(textkit.core, "normalize_spaces"):
    print("FAIL textkit.core has no normalize_spaces")
    sys.exit(1)
if "normalize_spaces" not in getattr(textkit, "__all__", []):
    print("FAIL 'normalize_spaces' missing from textkit.__all__")
    sys.exit(1)
if "do_the_thing" in getattr(textkit, "__all__", []):
    print("FAIL 'do_the_thing' still in textkit.__all__")
    sys.exit(1)

cases = [
    ("  hello\t world \n", "hello world"),
    ("", ""),
    ("one", "one"),
    ("a  b\tc\nd", "a b c d"),
    ("   \t \n", ""),
]
for text, want in cases:
    got = normalize_spaces(text)
    if got != want:
        print(f"FAIL normalize_spaces({text!r}) = {got!r}, expected {want!r}")
        sys.exit(1)

got = summarize(["  a  b ", "", "\t", "c"])
want = {"count": 2, "lines": ["a b", "c"]}
if got != want:
    print(f"FAIL summarize(...) = {got!r}, expected {want!r}")
    sys.exit(1)

print("rename verified, behavior unchanged")
PY
  then
    reward=1
  fi
fi

echo "$reward" > /logs/verifier/reward.txt
echo "[verifier] refactor-rename -> reward=$reward"
