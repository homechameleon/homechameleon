#!/usr/bin/env python3
"""Validate Holly plugin JSON and require every ${VAR} to be declared.

Scans this plugin directory only. Standard-library only, so CI needs no install.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLACEHOLDER = __import__("re").compile(r"\$\{([^{}]+)\}")
IDENTIFIER = __import__("re").compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
TEXT_SUFFIXES = {".json", ".md", ".yml", ".yaml", ".txt"}


def fail(messages: list[str]) -> None:
    for message in messages:
        print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def load_json(path: Path, errors: list[str]) -> object | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        errors.append(f"{path.relative_to(ROOT)}: invalid JSON ({exc})")
        return None


def declared_variables(manifest: object, errors: list[str]) -> set[str]:
    if not isinstance(manifest, dict):
        errors.append(".cursor-plugin/plugin.json: expected an object")
        return set()
    variables = manifest.get("variables")
    if not isinstance(variables, dict) or variables.get("type") != "object":
        errors.append("variables must be a JSON Schema object")
        return set()
    properties = variables.get("properties")
    if not isinstance(properties, dict) or not properties:
        errors.append("variables.properties must declare HA_URL and HA_TOKEN")
        return set()
    names: set[str] = set()
    for name, schema in properties.items():
        if not isinstance(name, str) or IDENTIFIER.fullmatch(name) is None:
            errors.append(f"variable name is not an identifier: {name!r}")
            continue
        if not isinstance(schema, dict):
            errors.append(f"variable {name} schema must be an object")
            continue
        names.add(name)
    required = variables.get("required")
    if not isinstance(required, list) or set(required) != names:
        errors.append("variables.required must list every declared variable")
    return names


def check_manifest(manifest: object, errors: list[str]) -> None:
    if not isinstance(manifest, dict):
        return
    if manifest.get("name") != "holly-for-home-assistant":
        errors.append("plugin name must be holly-for-home-assistant")
    if not isinstance(manifest.get("description"), str) or not manifest["description"].strip():
        errors.append("plugin description is required")
    if manifest.get("license") != "MIT":
        errors.append("plugin license must be MIT")
    for key, value in manifest.items():
        if isinstance(value, str) and (value.startswith("/") or "://" in value):
            errors.append(f"{key} must be a relative path, not an absolute path or URL")
    variables = manifest.get("variables")
    properties = variables.get("properties") if isinstance(variables, dict) else None
    if isinstance(properties, dict):
        token = properties.get("HA_TOKEN")
        if not isinstance(token, dict) or token.get("secret") is not True:
            errors.append("HA_TOKEN must be declared with secret true")
        url = properties.get("HA_URL")
        if not isinstance(url, dict):
            errors.append("HA_URL must be declared under variables")


def check_mcp(mcp: object, errors: list[str]) -> None:
    if not isinstance(mcp, dict):
        errors.append("mcp.json: expected an object")
        return
    servers = mcp.get("mcpServers")
    if not isinstance(servers, dict) or len(servers) != 1:
        errors.append("mcp.json must declare one remote MCP server")
        return
    server = next(iter(servers.values()))
    if not isinstance(server, dict):
        errors.append("MCP server entry must be an object")
        return
    if server.get("url") != "${HA_URL}/api/mcp":
        errors.append('MCP url must be "${HA_URL}/api/mcp"')
    headers = server.get("headers")
    if not isinstance(headers, dict) or headers.get("Authorization") != "Bearer ${HA_TOKEN}":
        errors.append('MCP Authorization header must be "Bearer ${HA_TOKEN}"')
    if "command" in server:
        errors.append("MCP server must be remote, not a local command")


def scan_placeholders(declared: set[str], errors: list[str]) -> None:
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or path.suffix.lower() not in TEXT_SUFFIXES:
            continue
        if path.name == "validate_plugin.py":
            continue
        text = path.read_text(encoding="utf-8")
        for match in PLACEHOLDER.finditer(text):
            name = match.group(1).strip()
            rel = path.relative_to(ROOT)
            if IDENTIFIER.fullmatch(name) is None or name not in declared:
                errors.append(f"{rel}: ${{{name}}} is not a declared variable")


def main() -> None:
    errors: list[str] = []
    json_paths = sorted(ROOT.rglob("*.json"))
    if not json_paths:
        fail(["no JSON files found"])

    parsed: dict[Path, object] = {}
    for path in json_paths:
        document = load_json(path, errors)
        if document is not None:
            parsed[path] = document

    manifest_path = ROOT / ".cursor-plugin" / "plugin.json"
    mcp_path = ROOT / "mcp.json"
    manifest = parsed.get(manifest_path)
    if manifest is None:
        errors.append("missing .cursor-plugin/plugin.json")
        declared: set[str] = set()
    else:
        declared = declared_variables(manifest, errors)
        check_manifest(manifest, errors)
    if mcp_path not in parsed:
        errors.append("missing mcp.json")
    else:
        check_mcp(parsed[mcp_path], errors)

    scan_placeholders(declared, errors)
    if errors:
        fail(errors)
    print(f"ok: {len(json_paths)} JSON files, variables {', '.join(sorted(declared))}")


if __name__ == "__main__":
    main()
