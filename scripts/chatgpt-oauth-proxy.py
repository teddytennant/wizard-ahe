#!/usr/bin/env python3
"""Local OpenAI-compatible proxy backed by a ChatGPT OAuth session.

Reads the Codex / ChatGPT subscription tokens stored by `codex login` in
``~/.codex/auth.json`` (or ``~/.wizard/chatgpt_oauth.json`` when present) and
re-serves them as a local OpenAI-compatible endpoint so any client — AHE's
evolve-agent AND the in-container wizard — can talk to OpenAI via your ChatGPT
subscription with no API key:

    LLM_BASE_URL=http://localhost:8089/v1   LLM_API_KEY=unused   model=gpt-5.2

Keeps the access token fresh (proactive refresh near JWT expiry, plus a forced
refresh after a 401). The client's Authorization header is ignored and replaced
with the fresh bearer.

Stdlib only. Forwards exclusively to api.openai.com (never an open proxy).
Streaming (SSE) is passed through unbuffered.

Caveats:
- Requires an active ChatGPT/Codex subscription session (`codex login`).
- Binding 0.0.0.0 (needed so Docker containers can reach it via the bridge IP)
  exposes a token-injecting proxy on your LAN. Use HOST=127.0.0.1 if only the
  host-side evolve-agent needs it.
"""

from __future__ import annotations

import base64
import json
import os
import sys
import threading
import time
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

UPSTREAM = "https://api.openai.com"  # /v1/... appended from the request path
TOKEN_ENDPOINT = "https://auth.openai.com/oauth/token"
CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann"  # Codex public OAuth client
EXPIRY_LEEWAY = 120
HOP_BY_HOP = {
    "connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
    "te", "trailers", "transfer-encoding", "upgrade", "content-length", "host",
}

_lock = threading.Lock()


def _token_paths() -> list[Path]:
    override = os.environ.get("CHATGPT_OAUTH_PATH")
    if override:
        return [Path(override)]
    home = Path.home()
    return [
        home / ".wizard" / "chatgpt_oauth.json",
        home / ".codex" / "auth.json",
    ]


def _resolve_path() -> Path:
    for path in _token_paths():
        if path.is_file():
            return path
    paths = ", ".join(str(p) for p in _token_paths())
    raise SystemExit(
        f"no ChatGPT OAuth session found (checked {paths}); run `codex login` first"
    )


def _b64url_json(segment: str) -> dict:
    pad = "=" * (-len(segment) % 4)
    return json.loads(base64.urlsafe_b64decode(segment + pad))


def _jwt_exp(token: str) -> int | None:
    try:
        return int(_b64url_json(token.split(".")[1]).get("exp"))
    except Exception:
        return None


def _load_raw(path: Path) -> dict:
    return json.loads(path.read_text())


def _extract_tokens(doc: dict) -> dict:
    if "access_token" in doc:
        return doc
    tokens = doc.get("tokens")
    if isinstance(tokens, dict) and tokens.get("access_token"):
        return tokens
    raise SystemExit("stored ChatGPT session is missing access_token")


def _load() -> tuple[Path, dict, dict]:
    path = _resolve_path()
    doc = _load_raw(path)
    tokens = _extract_tokens(doc)
    refresh = tokens.get("refresh_token")
    if not refresh:
        raise SystemExit(
            f"stored ChatGPT session at {path} has no refresh_token; run `codex login` again"
        )
    return path, doc, tokens


def _save(path: Path, doc: dict) -> None:
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(doc, indent=2))
    os.chmod(tmp, 0o600)
    os.replace(tmp, path)


def _write_tokens(path: Path, doc: dict, tokens: dict, refreshed: dict) -> None:
    tokens["access_token"] = refreshed["access_token"]
    if refreshed.get("refresh_token"):
        tokens["refresh_token"] = refreshed["refresh_token"]
    if "tokens" in doc:
        doc["tokens"] = tokens
        doc["last_refresh"] = time.strftime("%Y-%m-%dT%H:%M:%S.000000000Z", time.gmtime())
    else:
        doc.update(tokens)
    _save(path, doc)


def _refresh(path: Path, doc: dict, tokens: dict) -> dict:
    body = urllib.parse.urlencode(
        {
            "grant_type": "refresh_token",
            "client_id": CLIENT_ID,
            "refresh_token": tokens["refresh_token"],
        }
    ).encode()
    req = urllib.request.Request(
        TOKEN_ENDPOINT,
        data=body,
        headers={
            "Accept": "application/json",
            "Content-Type": "application/x-www-form-urlencoded",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        refreshed = json.loads(resp.read())
    _write_tokens(path, doc, tokens, refreshed)
    print("[chatgpt-proxy] refreshed access token", file=sys.stderr)
    return tokens


def fresh_bearer(force: bool = False) -> str:
    with _lock:
        path, doc, tokens = _load()
        exp = _jwt_exp(tokens["access_token"])
        if force or (exp is not None and exp <= time.time() + EXPIRY_LEEWAY):
            tokens = _refresh(path, doc, tokens)
        return tokens["access_token"]


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        pass

    def _proxy(self, method: str) -> None:
        length = int(self.headers.get("Content-Length", 0) or 0)
        payload = self.rfile.read(length) if length else None

        headers = {
            k: v for k, v in self.headers.items()
            if k.lower() not in HOP_BY_HOP and k.lower() != "authorization"
        }
        try:
            self._forward(method, payload, headers, fresh_bearer())
        except urllib.error.HTTPError as e:
            if e.code == 401:
                try:
                    self._forward(method, payload, headers, fresh_bearer(force=True))
                    return
                except urllib.error.HTTPError as e2:
                    e = e2
            self._relay_error(e)
        except Exception as e:  # noqa: BLE001
            self.send_error(502, f"upstream error: {e}")

    def _forward(self, method, payload, headers, bearer) -> None:
        url = UPSTREAM + self.path
        req = urllib.request.Request(url, data=payload, method=method)
        for k, v in headers.items():
            req.add_header(k, v)
        req.add_header("Authorization", f"Bearer {bearer}")
        with urllib.request.urlopen(req, timeout=600) as resp:
            self.send_response(resp.status)
            for k, v in resp.headers.items():
                if k.lower() not in HOP_BY_HOP:
                    self.send_header(k, v)
            self.send_header("Transfer-Encoding", "chunked")
            self.end_headers()
            while True:
                chunk = resp.read(8192)
                if not chunk:
                    break
                self.wfile.write(b"%X\r\n%s\r\n" % (len(chunk), chunk))
                self.wfile.flush()
            self.wfile.write(b"0\r\n\r\n")

    def _relay_error(self, e: urllib.error.HTTPError) -> None:
        body = e.read()
        self.send_response(e.code)
        self.send_header("Content-Type", e.headers.get("Content-Type", "application/json"))
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self._proxy("GET")

    def do_POST(self):
        self._proxy("POST")


def main() -> None:
    host = os.environ.get("HOST", "0.0.0.0")
    port = int(os.environ.get("PORT", "8089"))
    path = _resolve_path()
    fresh_bearer()
    print(
        f"[chatgpt-proxy] OpenAI-compatible -> {UPSTREAM}/v1 on http://{host}:{port}/v1",
        file=sys.stderr,
    )
    print(f"[chatgpt-proxy] token: {path}", file=sys.stderr)
    ThreadingHTTPServer((host, port), Handler).serve_forever()


if __name__ == "__main__":
    main()