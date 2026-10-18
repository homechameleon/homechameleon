#!/usr/bin/env python3
"""Tiny stand-in for the HomeChameleon integration's HTTP API (test use only).

Implements the frozen v1 contract:
  POST /api/homechameleon/generate   -> writes nothing here, returns the v1 response
  POST /api/homechameleon/preview    -> same shape, applied/default_set false
  GET  /api/homechameleon/styles     -> launch style list

Every request is appended to the JSON-lines file given as argv[2] so the
tests can inspect the method, path, headers and body. The listening port is
written to argv[3] once the server is ready.
"""
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TOKEN = "test-token-123"
STYLES = [
    {"id": "sheer", "name": "Sheer", "description": "Frosted, translucent cards"},
    {"id": "bubbles", "name": "Bubbles", "description": "Round, soft, playful"},
    {"id": "material", "name": "Material", "description": "Expressive tonal surfaces"},
    {"id": "starship", "name": "Starship Console", "description": "LCARS-inspired console"},
]

record_file = sys.argv[2]


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def _record(self, body):
        entry = {
            "method": self.command,
            "path": self.path,
            "authorization": self.headers.get("Authorization"),
            "content_type": self.headers.get("Content-Type"),
            "body": body,
        }
        with open(record_file, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(entry) + "\n")

    def _send(self, status, payload):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _authorized(self):
        return self.headers.get("Authorization") == f"Bearer {TOKEN}"

    def do_GET(self):
        self._record(None)
        if not self._authorized():
            return self._send(401, {"message": "Unauthorized"})
        if self.path == "/api/homechameleon/styles":
            return self._send(200, {"styles": STYLES})
        return self._send(404, {"message": "Not found"})

    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        raw = self.rfile.read(length).decode("utf-8")
        try:
            body = json.loads(raw)
        except ValueError:
            body = {"_invalid_json": raw[:200]}
        self._record(body)
        if not self._authorized():
            return self._send(401, {"message": "Unauthorized"})
        if self.path not in ("/api/homechameleon/generate", "/api/homechameleon/preview"):
            return self._send(404, {"message": "Not found"})
        if not isinstance(body, dict) or not (body.get("palette") or body.get("images")):
            return self._send(400, {"message": "palette or images is required"})

        if body.get("name") == "TooBig":
            return self._send(413, {"message": "Request Entity Too Large"})

        preview = self.path.endswith("/preview")
        slug = "".join(c for c in body.get("name", "theme").lower() if c.isalnum() or c in "_-")
        files = [f"themes/{slug}.yaml"]
        if body.get("background", {}).get("enabled") and body.get("images"):
            files.append(f"www/homechameleon/{slug}/bg-3f9a1c2e.webp")
        warnings = []
        if body.get("style") == "starship":
            warnings.append("Starship Console needs card-mod for the console frame")
        if body.get("name") == "WarnMe":
            warnings.append("profile override detected for user dave")
        self._send(200, {
            "theme": body.get("name"),
            "files": [] if preview else files,
            "palette": {"bg": "#1a1b26", "surface": "#24283b", "primary": "#7aa2f7"},
            "contrast": {"pass": True, "checks": [{"pair": "fg/surface", "ratio": 9.1, "pass": True}]},
            "applied": (not preview) and bool(body.get("apply")),
            "default_set": (not preview) and bool(body.get("set_default")),
            "warnings": warnings,
        })


server = ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler)
with open(sys.argv[3], "w", encoding="utf-8") as fh:
    fh.write(str(server.server_address[1]))
server.serve_forever()
