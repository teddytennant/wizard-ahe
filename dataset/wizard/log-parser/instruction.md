The file `/app/logs/app.log` contains application log lines in this format:

```
<timestamp> <LEVEL> <message>
```

- `<timestamp>` is `YYYY-MM-DDTHH:MM:SS` (e.g. `2026-06-01T08:00:12`).
- `<LEVEL>` is one of `DEBUG`, `INFO`, `WARNING`, `ERROR`.
- `<message>` is the rest of the line.
- Lines that do not match this format must be ignored.
- The lines are NOT guaranteed to be in chronological order.

Write a script at `/app/parse_logs.py` such that running

```
python3 /app/parse_logs.py
```

reads `/app/logs/app.log` and writes `/app/summary.json`: a JSON object with
exactly these keys:

- `"counts"`: an object mapping each level that appears in the log to the
  number of valid lines with that level.
- `"first_timestamp"`: the chronologically earliest timestamp among the valid
  lines, as the original string.
- `"last_timestamp"`: the chronologically latest timestamp, as the original
  string.
- `"top_error"`: the message that occurs most often among `ERROR` lines
  (ties broken by the lexicographically smallest message).

Counts must be JSON numbers. Compute everything from the log file — do not
hard-code the answers. Only `/app/parse_logs.py` (and the `/app/summary.json`
it produces) are required.
