#!/bin/bash
# Verifier for the json-merge task. Reward 1.0 iff /app/merge.py implements
# deep_merge with the specified dict/list/scalar semantics, without mutating
# inputs and without sharing structure with them.
mkdir -p /logs/verifier

reward=0
if [ -f /app/merge.py ]; then
  if python3 - <<'PY'
import copy
import sys

sys.path.insert(0, "/app")
try:
    from merge import deep_merge
except Exception as e:
    print("import failed:", e)
    sys.exit(1)


def check(got, want, desc):
    if got != want or type(got) is not type(want):
        print(f"FAIL {desc}: got {got!r}, expected {want!r}")
        sys.exit(1)


# Scalars: b wins.
check(deep_merge(1, 2), 2, "scalar int")
check(deep_merge("x", None), None, "scalar b=None")
check(deep_merge(True, "yes"), "yes", "scalar type change")

# Lists: concatenation.
check(deep_merge([1, 2], [3]), [1, 2, 3], "list concat")
check(deep_merge([], []), [], "empty lists")
check(deep_merge([[1]], [[2]]), [[1], [2]], "nested list concat")

# Dicts: recursive union merge.
check(
    deep_merge({"x": {"y": 1, "z": 2}, "k": [1]}, {"x": {"y": 9}, "m": 3}),
    {"x": {"y": 9, "z": 2}, "k": [1], "m": 3},
    "nested dict merge",
)
check(deep_merge({}, {"a": 1}), {"a": 1}, "empty a dict")
check(deep_merge({"a": 1}, {}), {"a": 1}, "empty b dict")
check(
    deep_merge({"a": {"b": [1]}}, {"a": {"b": [2]}}),
    {"a": {"b": [1, 2]}},
    "lists merged recursively inside dicts",
)

# Type conflicts: b wins wholesale.
check(deep_merge({"a": 1}, [1]), [1], "dict vs list")
check(deep_merge([1], {"a": 1}), {"a": 1}, "list vs dict")
check(deep_merge({"x": [1, 2]}, {"x": {"y": 2}}), {"x": {"y": 2}}, "conflict under key")
check(deep_merge({"x": {"y": 1}}, {"x": 5}), {"x": 5}, "dict replaced by scalar")

# Non-mutation.
a = {"x": {"y": 1}, "l": [1, 2]}
b = {"x": {"z": 2}, "l": [3]}
a_snapshot = copy.deepcopy(a)
b_snapshot = copy.deepcopy(b)
result = deep_merge(a, b)
check(result, {"x": {"y": 1, "z": 2}, "l": [1, 2, 3]}, "merge before mutation checks")
if a != a_snapshot:
    print(f"FAIL deep_merge mutated a: {a!r}")
    sys.exit(1)
if b != b_snapshot:
    print(f"FAIL deep_merge mutated b: {b!r}")
    sys.exit(1)

# No shared structure: mutating the result must not touch the inputs.
result["x"]["y"] = 999
result["x"]["z"] = 999
result["l"].append(999)
if a != a_snapshot or b != b_snapshot:
    print("FAIL result shares mutable structure with an input")
    sys.exit(1)

r2 = deep_merge({"only_a": {"deep": [1]}}, {"only_b": {"deep": [2]}})
r2["only_a"]["deep"].append(999)
r2["only_b"]["deep"].append(999)
# Rebuild fresh inputs to prove r2's mutations were isolated; nothing to assert
# on literals, so instead check aliasing directly on held references:
xa = {"deep": [1]}
xb = {"deep": [2]}
r3 = deep_merge({"k": xa}, {"m": xb})
r3["k"]["deep"].append(999)
r3["m"]["deep"].append(999)
if xa != {"deep": [1]} or xb != {"deep": [2]}:
    print("FAIL result aliases nested structures from the inputs")
    sys.exit(1)

print("all cases passed")
PY
  then
    reward=1
  fi
fi

echo "$reward" > /logs/verifier/reward.txt
echo "[verifier] json-merge -> reward=$reward"
