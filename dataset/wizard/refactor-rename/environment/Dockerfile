FROM wizard-ahe/task-base:latest

WORKDIR /app

COPY <<'EOF' /app/textkit/__init__.py
from .core import do_the_thing
from .report import summarize

__all__ = ["do_the_thing", "summarize"]
EOF

COPY <<'EOF' /app/textkit/core.py
def do_the_thing(text):
    """Collapse runs of whitespace into single spaces and strip the ends."""
    return " ".join(text.split())
EOF

COPY <<'EOF' /app/textkit/report.py
from .core import do_the_thing


def summarize(lines):
    """Return a summary of the non-blank lines after whitespace cleanup."""
    cleaned = [do_the_thing(line) for line in lines]
    cleaned = [line for line in cleaned if line]
    return {"count": len(cleaned), "lines": cleaned}
EOF
