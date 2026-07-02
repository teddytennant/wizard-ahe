The directory `/app/textkit` contains a small Python package:

- `textkit/__init__.py` — package exports
- `textkit/core.py` — defines the badly-named function `do_the_thing`
- `textkit/report.py` — uses that function

Rename `do_the_thing` to `normalize_spaces` everywhere in the package: the
definition in `core.py`, the re-export (including `__all__`) in `__init__.py`,
and every import and call site in `report.py`.

Hard constraints:

- Behavior must be completely unchanged — `normalize_spaces` must do exactly
  what `do_the_thing` did, and `summarize` must keep working.
- The old name must be gone: no alias, wrapper, or leftover reference. After
  the rename, the string `do_the_thing` must not appear in any `.py` file under
  `/app/textkit`, and neither `textkit` nor its modules may expose an attribute
  with the old name.
- Do not add, delete, or restructure files; only perform the rename.
