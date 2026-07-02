#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
cat > /app/stats_util.py <<'PY'
"""Tiny statistics helpers."""


def median(values):
    """Return the median of a non-empty list of numbers.

    Raises ValueError if the list is empty. Does not mutate the input.
    """
    if not values:
        raise ValueError("median() arg is an empty list")
    ordered = sorted(values)
    n = len(ordered)
    mid = n // 2
    if n % 2 == 1:
        return ordered[mid]
    return (ordered[mid - 1] + ordered[mid]) / 2
PY
