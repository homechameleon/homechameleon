#!/usr/bin/env python3
"""Validate Holly plugin JSON and require every ${VAR} to be declared.

Also checks the repository marketplace manifest: each entry's source must be a
folder under the repo root that contains .cursor-plugin/plugin.json, and that
manifest's name must match the entry. Standard-library only, so CI needs no install.
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


def check_marketplace(declared: set[str], errors: list[str]) -> int:
    """Return how many plugin entries were checked."""
    repo = ROOT.parent
    path = repo / ".cursor-plugin" / "marketplace.json"
    if not path.is_file():
        errors.append("missing .cursor-plugin/marketplace.json at the repository root")
        return 0
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        errors.append(f".cursor-plugin/marketplace.json: invalid JSON ({exc})")
        return 0
    if not isinstance(document, dict):
        errors.append(".cursor-plugin/marketplace.json: expected an object")
        return 0
    plugins = document.get("plugins")
    if not isinstance(plugins, list):
        errors.append("marketplace plugins must be a list")
        return 0
    repo_resolved = repo.resolve()
    checked = 0
    for index, entry in enumerate(plugins):
        label = f"plugins[{index}]"
        if not isinstance(entry, dict):
            errors.append(f"{label} must be an object")
            continue
        name = entry.get("name")
        source = entry.get("source")
        if isinstance(name, str) and name.strip():
            label = name.strip()
        else:
            errors.append(f"{label} needs a name")
            continue
        if isinstance(source, dict):
            source_value = source.get("path")
        else:
            source_value = source
        if (
            not isinstance(source_value, str)
            or not source_value.strip()
            or source_value.startswith("/")
            or "://" in source_value
        ):
            errors.append(f"{label}: source must be a relative folder")
            continue
        plugin_dir = (repo / source_value).resolve()
        try:
            plugin_dir.relative_to(repo_resolved)
        except ValueError:
            errors.append(f"{label}: source escapes the repository")
            continue
        manifest_path = plugin_dir / ".cursor-plugin" / "plugin.json"
        if not manifest_path.is_file():
            errors.append(f"{label}: {source_value} has no .cursor-plugin/plugin.json")
            continue
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            errors.append(f"{label}: plugin.json is invalid JSON ({exc})")
            continue
        found = manifest.get("name") if isinstance(manifest, dict) else None
        if found != label:
            errors.append(f"{label}: plugin.json name is {found!r}")
            continue
        checked += 1
    text = path.read_text(encoding="utf-8")
    for match in PLACEHOLDER.finditer(text):
        token = match.group(1).strip()
        if IDENTIFIER.fullmatch(token) is None or token not in declared:
            errors.append(f".cursor-plugin/marketplace.json: ${{{token}}} is not a declared variable")
    return checked


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
    marketplace_entries = check_marketplace(declared, errors)
    if errors:
        fail(errors)
    print(
        f"ok: {len(json_paths)} JSON files, "
        f"variables {', '.join(sorted(declared))}, "
        f"marketplace entries {marketplace_entries}"
    )


if __name__ == "__main__":
    main()
