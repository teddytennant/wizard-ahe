The file `/app/data/sales.csv` contains sales records. It has a header row:

```
date,category,item,quantity,unit_price
```

- `quantity` is an integer.
- `unit_price` is a decimal dollar amount.
- The revenue of a row is `quantity * unit_price`.

Compute the totals from the CSV and write a file `/app/report.json` containing a
single JSON object with exactly two keys:

- `"categories"`: an object mapping each category name that appears in the CSV
  to its total revenue (the sum of row revenues for that category), rounded to
  2 decimal places.
- `"grand_total"`: the sum of all row revenues, rounded to 2 decimal places.

Example of the required shape (values here are made up):

```json
{"categories": {"grocery": 12.5, "toys": 3.0}, "grand_total": 15.5}
```

All values must be JSON numbers, not strings. Compute the values from the CSV.
Only the file `/app/report.json` is required; no other output.
