#!/bin/bash
# Verifier for the csv-report task. Reward 1.0 iff /app/report.json contains the
# correct per-category totals and grand total for /app/data/sales.csv.
mkdir -p /logs/verifier

reward=0
if [ -f /app/report.json ]; then
  if python3 - <<'PY'
import json
import sys

expected_categories = {
    "electronics": 115.99,  # 2*25.50 + 1*49.99 + 4*3.75
    "grocery": 14.75,       # 10*0.50 + 3*2.25 + 2*1.50
    "toys": 45.98,          # 2*19.99 + 5*1.20
}
expected_grand = 176.72

try:
    with open("/app/report.json") as f:
        report = json.load(f)
except Exception as e:
    print("failed to load /app/report.json:", e)
    sys.exit(1)

def is_number(x):
    return isinstance(x, (int, float)) and not isinstance(x, bool)

if not isinstance(report, dict) or set(report) != {"categories", "grand_total"}:
    print("top-level object must have exactly the keys 'categories' and 'grand_total', got:", report)
    sys.exit(1)

cats = report["categories"]
if not isinstance(cats, dict) or set(cats) != set(expected_categories):
    print("'categories' must map exactly the categories in the CSV, got:", cats)
    sys.exit(1)

for name, want in expected_categories.items():
    got = cats[name]
    if not is_number(got) or abs(got - want) > 0.005:
        print(f"FAIL categories[{name!r}] = {got!r}, expected {want}")
        sys.exit(1)

grand = report["grand_total"]
if not is_number(grand) or abs(grand - expected_grand) > 0.005:
    print(f"FAIL grand_total = {grand!r}, expected {expected_grand}")
    sys.exit(1)

print("report.json is correct")
PY
  then
    reward=1
  fi
fi

echo "$reward" > /logs/verifier/reward.txt
echo "[verifier] csv-report -> reward=$reward"
