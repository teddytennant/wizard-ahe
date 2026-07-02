FROM wizard-ahe/task-base:latest

WORKDIR /app

COPY <<'EOF' /app/stats_util.py
"""Tiny statistics helpers."""


def median(values):
    """Return the median of a non-empty list of numbers.

    Raises ValueError if the list is empty. Does not mutate the input.
    """
    if not values:
        raise ValueError("median() arg is an empty list")
    ordered = sorted(values)
    return ordered[len(ordered) // 2]
EOF

COPY <<'EOF' /app/test_stats_util.py
import pytest

from stats_util import median


def test_odd_length():
    assert median([3, 1, 2]) == 2


def test_even_length_is_mean_of_middle_two():
    assert median([1, 2, 3, 4]) == 2.5


def test_even_length_unsorted():
    assert median([9, 1]) == 5.0


def test_single_element():
    assert median([7]) == 7


def test_empty_raises():
    with pytest.raises(ValueError):
        median([])


def test_does_not_mutate_input():
    values = [3, 1, 2]
    median(values)
    assert values == [3, 1, 2]
EOF
