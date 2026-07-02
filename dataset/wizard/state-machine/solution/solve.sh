#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
cat > /app/turnstile.py <<'PY'
class Turnstile:
    """A subway turnstile: locked/unlocked, driven by coin/push events."""

    _TRANSITIONS = {
        ("locked", "coin"): "unlocked",
        ("locked", "push"): "locked",
        ("unlocked", "coin"): "unlocked",
        ("unlocked", "push"): "locked",
    }

    def __init__(self):
        self.state = "locked"
        self.log = []

    def handle(self, event):
        # Membership test (==) rather than dict lookup: unhashable values such
        # as lists must also raise ValueError, not TypeError.
        if event not in ("coin", "push"):
            raise ValueError(f"invalid event: {event!r}")
        self.state = self._TRANSITIONS[(self.state, event)]
        self.log.append((event, self.state))
        return self.state
PY
