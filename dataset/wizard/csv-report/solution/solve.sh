#!/bin/bash
# Reference solution (harbor's OracleAgent uses it to confirm the task is solvable).
set -e
python3 - <<'PY'
import csv
import json

totals = {}
with open("/app/data/sales.csv", newline="") as f:
    for row in csv.DictReader(f):
        revenue = int(row["quantity"]) * float(row["unit_price"])
        totals[row["category"]] = totals.get(row["category"], 0.0) + revenue

report = {
    "categories": {k: round(v, 2) for k, v in totals.items()},
    "grand_total": round(sum(totals.values()), 2),
}
with open("/app/report.json", "w") as f:
    json.dump(report, f, indent=2)
PY
