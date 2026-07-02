#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
cat > /app/wordfreq.py <<'PY'
import argparse
import re
import sys
from collections import Counter


def main():
    parser = argparse.ArgumentParser(description="Print the most frequent words in a file.")
    parser.add_argument("file")
    parser.add_argument("--top", type=int, default=10)
    parser.add_argument("--min-len", type=int, default=1, dest="min_len")
    args = parser.parse_args()

    try:
        with open(args.file, encoding="utf-8") as f:
            text = f.read()
    except OSError as e:
        print(f"error: cannot read {args.file}: {e}", file=sys.stderr)
        sys.exit(1)

    words = [w.lower() for w in re.findall(r"[A-Za-z]+", text)]
    counts = Counter(w for w in words if len(w) >= args.min_len)
    ranked = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    for word, count in ranked[: args.top]:
        print(f"{word} {count}")


if __name__ == "__main__":
    main()
PY
