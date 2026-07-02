The directory `/app` contains a small Python module and its test suite:

- `/app/stats_util.py` — the module under test
- `/app/test_stats_util.py` — pytest tests (pytest is already installed)

Running the tests currently fails:

```
cd /app && python3 -m pytest -q
```

There is a bug in `stats_util.py`. Fix the module so that the whole test suite
passes.

Hard constraints:

- You must NOT modify, rename, delete, or replace `/app/test_stats_util.py` in
  any way — the verifier checks its checksum. Fix the module, not the tests.
- `median` must keep its name and general contract (raise `ValueError` on an
  empty list, work for any non-empty list of numbers, not mutate its input).
  The fix must be a real implementation, not special-casing the test inputs.
