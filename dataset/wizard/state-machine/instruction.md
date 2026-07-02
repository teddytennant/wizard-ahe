Create a Python module at `/app/turnstile.py` that defines a class `Turnstile`
modeling a subway turnstile:

- `Turnstile()` takes no arguments and starts in state `"locked"`.
- `t.state` is the current state: the string `"locked"` or `"unlocked"`.
- `t.log` is a list of `(event, resulting_state)` tuples, one entry per
  successfully handled event, in order. A new instance starts with an empty
  log, and every instance must have its own independent log.
- `t.handle(event)` processes an event and returns the resulting state string.
  The only valid events are the strings `"coin"` and `"push"`:

  | current state | event  | resulting state |
  |---------------|--------|-----------------|
  | locked        | coin   | unlocked        |
  | locked        | push   | locked          |
  | unlocked      | coin   | unlocked        |
  | unlocked      | push   | locked          |

  After handling a valid event, `(event, resulting_state)` is appended to
  `t.log`.

- For any other event value (any other string, `None`, a number, ...),
  `handle` must raise `ValueError` and leave both `state` and `log` unchanged.

Only the file `/app/turnstile.py` is required; no other output.
