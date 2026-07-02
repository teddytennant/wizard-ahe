Create a Python module at `/app/merge.py` that defines a function:

```python
def deep_merge(a, b):
    ...
```

`deep_merge` merges two JSON-like values (nested dicts, lists, and scalars such
as numbers, strings, booleans, `None`) and returns the merged value. Semantics:

- If `a` and `b` are both dicts: the result is a dict containing every key from
  either input. For keys present in both, the value is `deep_merge(a[k], b[k])`
  (applied recursively). Keys present in only one input keep that input's value.
- If `a` and `b` are both lists: the result is the concatenation `a` then `b`.
- In every other case (two scalars, or any type mismatch such as dict vs list,
  list vs scalar, ...): the result is `b`. `b` always wins.

Hard constraints:

- `deep_merge` must not mutate either argument.
- The returned value must not share any mutable structure (dicts/lists) with
  either input: mutating the result afterwards must never change `a` or `b`.

Only the file `/app/merge.py` is required; no other output.
