#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
cat > /app/parse_logs.py <<'PY'
import json
import re
from collections import Counter

LINE_RE = re.compile(
    r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}) (DEBUG|INFO|WARNING|ERROR) (.*)$"
)


def main():
    counts = Counter()
    timestamps = []
    errors = Counter()

    with open("/app/logs/app.log", encoding="utf-8") as f:
        for line in f:
            match = LINE_RE.match(line.rstrip("\n"))
            if not match:
                continue
            timestamp, level, message = match.groups()
            counts[level] += 1
            timestamps.append(timestamp)
            if level == "ERROR":
                errors[message] += 1

    top_error = min(errors, key=lambda msg: (-errors[msg], msg))
    summary = {
        "counts": dict(counts),
        "first_timestamp": min(timestamps),
        "last_timestamp": max(timestamps),
        "top_error": top_error,
    }
    with open("/app/summary.json", "w") as f:
        json.dump(summary, f, indent=2)


if __name__ == "__main__":
    main()
PY
python3 /app/parse_logs.py
