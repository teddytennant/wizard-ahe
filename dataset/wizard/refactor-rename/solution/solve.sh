#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
cd /app/textkit
sed -i 's/do_the_thing/normalize_spaces/g' __init__.py core.py report.py
