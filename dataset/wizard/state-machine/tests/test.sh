#!/bin/bash
# Verifier for the state-machine task. Drives Turnstile through valid and
# invalid event sequences, checks transitions, return values, the event log,
# and per-instance log isolation.
mkdir -p /logs/verifier

reward=0
if [ -f /app/turnstile.py ]; then
  if python3 - <<'PY'
import sys

sys.path.insert(0, "/app")
try:
    from turnstile import Turnstile
except Exception as e:
    print("import failed:", e)
    sys.exit(1)


def fail(msg):
    print("FAIL", msg)
    sys.exit(1)


# Initial state.
t = Turnstile()
if t.state != "locked":
    fail(f"initial state is {t.state!r}, expected 'locked'")
if list(t.log) != []:
    fail(f"initial log is {t.log!r}, expected []")

# Full valid sequence; handle() must return the resulting state.
sequence = [
    ("push", "locked"),     # push while locked stays locked
    ("coin", "unlocked"),
    ("coin", "unlocked"),   # coin while unlocked stays unlocked
    ("push", "locked"),
    ("coin", "unlocked"),
    ("push", "locked"),
    ("push", "locked"),
]
for event, want_state in sequence:
    got = t.handle(event)
    if got != want_state:
        fail(f"handle({event!r}) returned {got!r}, expected {want_state!r}")
    if t.state != want_state:
        fail(f"state is {t.state!r} after handle({event!r}), expected {want_state!r}")

if list(t.log) != sequence:
    fail(f"log is {t.log!r}, expected {sequence!r}")

# Invalid events raise ValueError and leave state/log untouched.
for bad in ("kick", "", "COIN", None, 3, ["coin"]):
    state_before = t.state
    log_before = list(t.log)
    try:
        t.handle(bad)
    except ValueError:
        pass
    except Exception as e:
        fail(f"handle({bad!r}) raised {type(e).__name__}, expected ValueError")
    else:
        fail(f"handle({bad!r}) did not raise ValueError")
    if t.state != state_before:
        fail(f"invalid event {bad!r} changed state to {t.state!r}")
    if list(t.log) != log_before:
        fail(f"invalid event {bad!r} changed the log")

# Invalid event mid-sequence, then recovery.
t2 = Turnstile()
t2.handle("coin")
try:
    t2.handle("jump")
except ValueError:
    pass
else:
    fail("handle('jump') did not raise ValueError")
if t2.state != "unlocked":
    fail("invalid event disturbed t2.state")
if t2.handle("push") != "locked":
    fail("t2 did not recover after an invalid event")

# Instances must not share logs (mutable class attribute trap).
a = Turnstile()
b = Turnstile()
a.handle("coin")
if list(b.log) != [] or b.state != "locked":
    fail("instances share state or log")
if list(a.log) != [("coin", "unlocked")]:
    fail(f"a.log is {a.log!r}, expected [('coin', 'unlocked')]")

print("all sequences passed")
PY
  then
    reward=1
  fi
fi

echo "$reward" > /logs/verifier/reward.txt
echo "[verifier] state-machine -> reward=$reward"
