#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
mkdir -p /app/repo
cd /app/repo
git init -q
git config user.name "Eval Bot"
git config user.email "eval@example.com"

for n in 1 2 3; do
  printf '%s\n' "$n" > "file$n.txt"
  git add "file$n.txt"
  git commit -q -m "step $n"
done
