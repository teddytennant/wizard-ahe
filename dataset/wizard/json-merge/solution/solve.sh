#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
cat > /app/merge.py <<'PY'
import copy


def deep_merge(a, b):
    """Merge two JSON-like values.

    dict + dict -> recursive key-union merge (b wins on leaf conflicts)
    list + list -> concatenation a + b
    anything else (scalars or type mismatch) -> b

    Never mutates the inputs; the result shares no mutable structure with them.
    """
    if isinstance(a, dict) and isinstance(b, dict):
        merged = {}
        for key, value in a.items():
            if key in b:
                merged[key] = deep_merge(value, b[key])
            else:
                merged[key] = copy.deepcopy(value)
        for key, value in b.items():
            if key not in a:
                merged[key] = copy.deepcopy(value)
        return merged
    if isinstance(a, list) and isinstance(b, list):
        return copy.deepcopy(a) + copy.deepcopy(b)
    return copy.deepcopy(b)
PY
