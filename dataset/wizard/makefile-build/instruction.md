The directory `/app/src` contains text files (`*.txt`). Create a Makefile at
`/app/Makefile` (GNU make is installed) with exactly these three targets:

- `build` — creates `dist/` if needed and writes `dist/bundle.txt`: the
  concatenation of the contents of ALL `.txt` files in `src/`, in sorted
  (lexicographic) filename order. It must not hard-code the current filenames:
  if a `.txt` file is later added to or removed from `src/`, a fresh
  `make build` must reflect that. Re-running `build` overwrites the bundle.
- `test` — exits with a non-zero status if `dist/bundle.txt` is missing OR its
  content differs from the sorted concatenation of the current `src/*.txt`
  files; exits with status `0` otherwise. It must not create or modify any
  files.
- `clean` — removes the `dist` directory (and nothing else). It must succeed
  (exit `0`) even when `dist` does not exist.

`build` must be the default target (a plain `make` runs it). Paths are relative
to `/app`, and the targets must work when `make` is invoked from `/app`. Do not
modify the files in `src/`.
