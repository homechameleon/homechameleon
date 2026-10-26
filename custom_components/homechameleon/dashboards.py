"""Dashboard snapshots and per-view theme assignment.

HomeChameleon's own files stay under www/homechameleon/. Changing a view
goes through Lovelace's storage API. YAML dashboards are left untouched.
"""

from __future__ import annotations

import copy
import json
import secrets
from datetime import UTC, datetime
from typing import Any

from homeassistant.components.frontend import (
    DATA_DEFAULT_DARK_THEME,
    DATA_DEFAULT_THEME,
    DEFAULT_THEME,
)
from homeassistant.components.frontend.storage import async_user_store
from homeassistant.components.lovelace import LovelaceData
from homeassistant.components.lovelace.const import (
    LOVELACE_DATA,
    MODE_STORAGE,
    ConfigNotFound,
)
from homeassistant.components.lovelace.dashboard import LovelaceConfig
from homeassistant.core import HomeAssistant
from homeassistant.exceptions import HomeAssistantError

from .const import MAX_SNAPSHOT_BYTES
from .files import PathError, atomic_write, snapshot_file, undo_file, www_root

SNAPSHOT_SCHEMA = "homechameleon-snapshot/1"
UNDO_SCHEMA = "homechameleon-undo/1"
_NO_DARK_THEME = "none"


class BindError(Exception):
    """A view target or a snapshot could not be used."""

    def __init__(self, message: str, status: int = 400) -> None:
        super().__init__(message)
        self.status = status


def parse_view_target(body: dict[str, Any]) -> dict[str, Any] | None:
    """Return a view target from a request body, or None when it is absent."""
    if "view" not in body or body["view"] is None:
        return None
    raw = body["view"]
    if not isinstance(raw, dict):
        raise BindError("view must be an object")
    url_path = raw.get("url_path")
    if not isinstance(url_path, str) or not url_path.strip():
        raise BindError("view url_path is required")
    path = raw.get("path")
    index = raw.get("index")
    if path is not None and not isinstance(path, str):
        raise BindError("view path must be a string")
    if isinstance(path, str):
        path = path.strip() or None
    if index is not None and (isinstance(index, bool) or not isinstance(index, int)):
        raise BindError("view index must be an integer")
    if path is None and index is None:
        raise BindError("view path or index is required")
    return {"url_path": url_path.strip(), "path": path, "index": index}


def _lovelace(hass: HomeAssistant) -> LovelaceData:
    data = hass.data.get(LOVELACE_DATA)
    if data is None:
        raise BindError("Dashboards are not available.", 400)
    return data


def _dashboard(hass: HomeAssistant, url_path: str) -> LovelaceConfig:
    data = _lovelace(hass)
    if url_path in data.dashboards:
        return data.dashboards[url_path]
    for board in data.dashboards.values():
        if board.url_path == url_path:
            return board
    if url_path == "lovelace" and None in data.dashboards:
        board = data.dashboards[None]
        if board.url_path in {None, "lovelace"}:
            return board
    raise BindError("dashboard not found", 404)


def _board_title(board: LovelaceConfig) -> str:
    if board.config and isinstance(board.config.get("title"), str) and board.config["title"].strip():
        return board.config["title"].strip()
    return "Dashboard"


def _public_url_path(board: LovelaceConfig, requested: str | None = None) -> str:
    if isinstance(requested, str) and requested:
        return requested
    if isinstance(board.url_path, str) and board.url_path:
        return board.url_path
    return "lovelace"


def _view_index(config: dict[str, Any], *, path: str | None, index: int | None) -> int:
    views = config.get("views")
    if not isinstance(views, list) or not views:
        raise BindError("This dashboard has no views.", 400)
    if path:
        matches = [
            i
            for i, view in enumerate(views)
            if isinstance(view, dict) and view.get("path") == path
        ]
        if len(matches) == 1:
            return matches[0]
        if len(matches) > 1:
            raise BindError("That view path is used more than once.", 400)
        raise BindError("dashboard view not found", 404)
    if index is None:
        raise BindError("view path or index is required", 400)
    if index < 0 or index >= len(views) or not isinstance(views[index], dict):
        raise BindError("dashboard view not found", 404)
    return index


def _theme_state(view: dict[str, Any]) -> tuple[bool, str | None]:
    if "theme" not in view:
        return False, None
    raw = view.get("theme")
    if isinstance(raw, str):
        return True, raw
    return True, None


def _apply_theme_state(view: dict[str, Any], present: bool, theme: str | None) -> None:
    if present:
        view["theme"] = theme
    else:
        view.pop("theme", None)


async def _load_config(board: LovelaceConfig) -> dict[str, Any]:
    try:
        config = await board.async_load(False)
    except ConfigNotFound as err:
        raise BindError("This dashboard has no saved layout yet.", 400) from err
    if not isinstance(config, dict):
        raise BindError("This dashboard has no saved layout yet.", 400)
    return config


async def async_check_view(hass: HomeAssistant, view: dict[str, Any]) -> None:
    """Fail before any theme file is written when the view cannot be updated."""
    board = _dashboard(hass, view["url_path"])
    if board.mode != MODE_STORAGE:
        raise BindError(
            "This dashboard is stored as YAML. HomeChameleon does not edit that file.",
            400,
        )
    config = await _load_config(board)
    _view_index(config, path=view.get("path"), index=view.get("index"))


def _stamp() -> str:
    return datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _new_snapshot_id(url_path: str) -> str:
    stem = "".join(char for char in url_path.lower() if char.isalnum() or char in "-_")[:24] or "dashboard"
    stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%S%fZ").lower()
    return f"{stem}-{stamp}-{secrets.token_hex(3)}"


def _global_theme(hass: HomeAssistant) -> dict[str, str | None]:
    name = hass.data.get(DATA_DEFAULT_THEME, DEFAULT_THEME)
    if not isinstance(name, str) or not name:
        name = DEFAULT_THEME
    dark = hass.data.get(DATA_DEFAULT_DARK_THEME)
    if not isinstance(dark, str) or not dark:
        dark = None
    return {"name": name, "name_dark": dark}


async def _user_themes(hass: HomeAssistant) -> tuple[list[dict[str, Any]], list[str]]:
    rows: list[dict[str, Any]] = []
    warnings: list[str] = []
    for user in await hass.auth.async_get_users():
        try:
            store = await async_user_store(hass, user.id)
            theme = copy.deepcopy(store.data.get("theme")) if "theme" in store.data else None
        except Exception as err:  # noqa: BLE001 - keep going, the view change can still be undone
            warnings.append(f"could not read a saved theme for one user: {err}")
            continue
        rows.append({"user_id": user.id, "theme": theme})
    return rows, warnings


def _write_json(path, document: dict[str, Any]) -> None:
    encoded = (json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if len(encoded) > MAX_SNAPSHOT_BYTES:
        raise BindError("dashboard snapshot is too large", 400)
    atomic_write(path, encoded)


def _read_json(path) -> dict[str, Any] | None:
    if not path.is_file() or path.is_symlink():
        return None
    try:
        parsed = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    if not isinstance(parsed, dict):
        return None
    return parsed


async def _write_snapshot(
    hass: HomeAssistant,
    *,
    url_path: str,
    title: str,
    mode: str,
    config: dict[str, Any],
    reason: str,
    theme: str | None,
    view: dict[str, Any] | None,
) -> str:
    snapshot_id = _new_snapshot_id(url_path)
    document = {
        "schema": SNAPSHOT_SCHEMA,
        "id": snapshot_id,
        "created": _stamp(),
        "url_path": url_path,
        "title": title,
        "mode": mode,
        "reason": reason,
        "theme": theme,
        "view": view,
        "config": config,
    }
    path = snapshot_file(hass.config.config_dir, snapshot_id)
    try:
        await hass.async_add_executor_job(_write_json, path, document)
    except PathError as err:
        raise BindError(str(err), 400) from err
    return snapshot_id


def _summary(document: dict[str, Any]) -> dict[str, Any]:
    config = document.get("config")
    views = config.get("views") if isinstance(config, dict) else None
    return {
        "id": document.get("id"),
        "created": document.get("created"),
        "url_path": document.get("url_path"),
        "title": document.get("title"),
        "mode": document.get("mode"),
        "reason": document.get("reason"),
        "theme": document.get("theme"),
        "views": len(views) if isinstance(views, list) else 0,
    }


def _list_documents(config_dir: str) -> list[dict[str, Any]]:
    folder = www_root(config_dir) / "snapshots"
    if not folder.is_dir() or folder.is_symlink():
        return []
    found: list[dict[str, Any]] = []
    for path in folder.glob("*.json"):
        if not path.is_file() or path.is_symlink():
            continue
        if path.name.startswith("."):
            continue
        document = _read_json(path)
        if document is None or document.get("schema") != SNAPSHOT_SCHEMA:
            continue
        if document.get("id") != path.stem:
            continue
        found.append(document)
    found.sort(key=lambda item: str(item.get("created") or ""), reverse=True)
    return found


async def async_list_snapshots(hass: HomeAssistant) -> list[dict[str, Any]]:
    """Return snapshot summaries, newest first."""
    documents = await hass.async_add_executor_job(_list_documents, hass.config.config_dir)
    return [_summary(item) for item in documents]


async def async_get_snapshot(hass: HomeAssistant, snapshot_id: str) -> dict[str, Any]:
    """Return one snapshot document."""
    try:
        path = snapshot_file(hass.config.config_dir, snapshot_id)
    except PathError as err:
        raise BindError(str(err), 400) from err
    document = await hass.async_add_executor_job(_read_json, path)
    if document is None or document.get("schema") != SNAPSHOT_SCHEMA or document.get("id") != snapshot_id:
        raise BindError("snapshot not found", 404)
    return document


async def async_create_snapshot(hass: HomeAssistant, url_path: str) -> dict[str, Any]:
    """Copy a dashboard config into www/homechameleon/snapshots/."""
    if not isinstance(url_path, str) or not url_path.strip():
        raise BindError("url_path is required", 400)
    board = _dashboard(hass, url_path.strip())
    config = copy.deepcopy(await _load_config(board))
    public_path = _public_url_path(board, url_path.strip())
    snapshot_id = await _write_snapshot(
        hass,
        url_path=public_path,
        title=_board_title(board),
        mode=board.mode,
        config=config,
        reason="manual",
        theme=None,
        view=None,
    )
    document = await async_get_snapshot(hass, snapshot_id)
    return _summary(document)


async def async_restore_snapshot(hass: HomeAssistant, snapshot_id: str) -> dict[str, Any]:
    """Put a stored dashboard config back. YAML files are not written."""
    document = await async_get_snapshot(hass, snapshot_id)
    url_path = document.get("url_path")
    config = document.get("config")
    if not isinstance(url_path, str) or not isinstance(config, dict):
        raise BindError("snapshot could not be read", 400)
    board = _dashboard(hass, url_path)
    if board.mode != MODE_STORAGE:
        raise BindError(
            "This dashboard is stored as YAML. HomeChameleon does not edit that file.",
            400,
        )
    try:
        await board.async_save(copy.deepcopy(config))
    except HomeAssistantError as err:
        raise BindError(str(err), 400) from err
    return {"id": snapshot_id, "url_path": url_path, "restored": True}


async def async_delete_snapshot(hass: HomeAssistant, snapshot_id: str) -> None:
    """Delete one snapshot file."""
    try:
        path = snapshot_file(hass.config.config_dir, snapshot_id)
    except PathError as err:
        raise BindError(str(err), 400) from err

    def _delete() -> bool:
        if path.is_symlink():
            raise PathError("snapshot name is not allowed")
        if not path.is_file():
            return False
        path.unlink()
        return True

    try:
        found = await hass.async_add_executor_job(_delete)
    except PathError as err:
        raise BindError(str(err), 400) from err
    if not found:
        raise BindError("snapshot not found", 404)


def _view_rows(config: dict[str, Any]) -> list[dict[str, Any]]:
    views = config.get("views")
    if not isinstance(views, list):
        return []
    rows: list[dict[str, Any]] = []
    for index, view in enumerate(views):
        if not isinstance(view, dict):
            continue
        path = view.get("path") if isinstance(view.get("path"), str) and view.get("path") else None
        title = view.get("title") if isinstance(view.get("title"), str) and view.get("title").strip() else None
        present, theme = _theme_state(view)
        rows.append(
            {
                "index": index,
                "path": path,
                "title": title or (path or f"View {index + 1}"),
                "theme": theme if present else None,
            }
        )
    return rows


async def async_list_dashboards(hass: HomeAssistant) -> list[dict[str, Any]]:
    """Return dashboards and the theme currently set on each view."""
    data = hass.data.get(LOVELACE_DATA)
    if data is None:
        return []
    rows: list[dict[str, Any]] = []
    for board in data.dashboards.values():
        url_path = _public_url_path(board)
        item: dict[str, Any] = {
            "url_path": url_path,
            "title": _board_title(board),
            "mode": board.mode,
            "editable": board.mode == MODE_STORAGE,
            "views": [],
        }
        try:
            config = await board.async_load(False)
        except ConfigNotFound:
            item["note"] = "This dashboard has no saved layout yet."
        except Exception:  # noqa: BLE001 - one unreadable dashboard should not hide the others
            item["note"] = "This dashboard could not be read."
        else:
            if isinstance(config, dict):
                item["views"] = _view_rows(config)
            else:
                item["note"] = "This dashboard has no saved layout yet."
        rows.append(item)
    rows.sort(key=lambda item: (item["title"].lower(), item["url_path"]))
    return rows


async def _write_undo(hass: HomeAssistant, document: dict[str, Any]) -> None:
    path = undo_file(hass.config.config_dir)
    try:
        await hass.async_add_executor_job(_write_json, path, document)
    except PathError as err:
        raise BindError(str(err), 400) from err


async def _read_undo(hass: HomeAssistant) -> dict[str, Any] | None:
    path = undo_file(hass.config.config_dir)
    document = await hass.async_add_executor_job(_read_json, path)
    if document is None or document.get("schema") != UNDO_SCHEMA:
        return None
    return document


def _undo_public(document: dict[str, Any]) -> dict[str, Any]:
    view = document.get("view")
    public_view = None
    if isinstance(view, dict):
        public_view = {
            "url_path": view.get("url_path"),
            "path": view.get("path"),
            "index": view.get("index"),
        }
    return {"available": True, "theme": document.get("theme"), "view": public_view}


async def async_undo_status(hass: HomeAssistant) -> dict[str, Any]:
    """Return whether a one-click theme restore is waiting."""
    document = await _read_undo(hass)
    if document is None:
        return {"available": False}
    return _undo_public(document)


async def _capture_undo(
    hass: HomeAssistant,
    *,
    theme: str,
    view_state: dict[str, Any] | None,
) -> tuple[dict[str, Any], list[str]]:
    users, warnings = await _user_themes(hass)
    document = {
        "schema": UNDO_SCHEMA,
        "created": _stamp(),
        "theme": theme,
        "global": _global_theme(hass),
        "users": users,
        "view": view_state,
    }
    await _write_undo(hass, document)
    return document, warnings


async def _assign_view(
    hass: HomeAssistant, view: dict[str, Any], theme: str
) -> tuple[str, dict[str, Any], list[str]]:
    """Snapshot the dashboard, record themes, then set the view theme."""
    board = _dashboard(hass, view["url_path"])
    if board.mode != MODE_STORAGE:
        raise BindError(
            "This dashboard is stored as YAML. HomeChameleon does not edit that file.",
            400,
        )
    original = copy.deepcopy(await _load_config(board))
    index = _view_index(original, path=view.get("path"), index=view.get("index"))
    current = original["views"][index]
    present, previous = _theme_state(current)
    path = current.get("path") if isinstance(current.get("path"), str) and current.get("path") else None
    url_path = _public_url_path(board, view["url_path"])
    view_state = {
        "url_path": url_path,
        "index": index,
        "path": path,
        "theme_present": present,
        "theme": previous,
    }
    snapshot_id = await _write_snapshot(
        hass,
        url_path=url_path,
        title=_board_title(board),
        mode=board.mode,
        config=original,
        reason="before_view_theme",
        theme=theme,
        view={"path": path, "index": index},
    )
    _document, notes = await _capture_undo(hass, theme=theme, view_state=view_state)
    updated = copy.deepcopy(original)
    updated["views"][index]["theme"] = theme
    try:
        await board.async_save(updated)
    except HomeAssistantError as err:
        raise BindError(str(err), 400) from err
    return snapshot_id, {
        "url_path": url_path,
        "path": path,
        "index": index,
        "theme": theme,
    }, notes


async def async_finish_apply(
    hass: HomeAssistant,
    *,
    theme: str,
    view: dict[str, Any] | None,
    set_default: bool,
) -> dict[str, Any]:
    """Record the previous themes, then assign a view theme and the optional default.

    A view change snapshots the dashboard first. set_default stays off unless
    the caller asks for it. The undo record is written before either change.
    """
    warnings: list[str] = []
    extra: dict[str, Any] = {}
    if view is not None:
        snapshot_id, public_view, notes = await _assign_view(hass, view, theme)
        warnings.extend(notes)
        extra["snapshot"] = snapshot_id
        extra["view"] = public_view
        extra["undo"] = True
    elif set_default:
        _document, notes = await _capture_undo(hass, theme=theme, view_state=None)
        warnings.extend(notes)
        extra["undo"] = True
    default_set = False
    if set_default:
        from .files import async_set_default_theme

        note = await async_set_default_theme(hass, theme)
        if note:
            warnings.append(note)
        else:
            default_set = True
    return {"default_set": default_set, "warnings": warnings, **extra}


async def _restore_view(hass: HomeAssistant, view: dict[str, Any]) -> None:
    board = _dashboard(hass, str(view.get("url_path") or ""))
    if board.mode != MODE_STORAGE:
        raise BindError(
            "This dashboard is stored as YAML. HomeChameleon does not edit that file.",
            400,
        )
    config = copy.deepcopy(await _load_config(board))
    path = view.get("path") if isinstance(view.get("path"), str) else None
    index = view.get("index") if isinstance(view.get("index"), int) and not isinstance(view.get("index"), bool) else None
    found = _view_index(config, path=path, index=index)
    present = bool(view.get("theme_present"))
    theme = view.get("theme") if isinstance(view.get("theme"), str) else None
    _apply_theme_state(config["views"][found], present, theme)
    try:
        await board.async_save(config)
    except HomeAssistantError as err:
        raise BindError(str(err), 400) from err


async def _restore_users(hass: HomeAssistant, users: list[Any]) -> list[str]:
    warnings: list[str] = []
    if not isinstance(users, list):
        return ["saved user themes could not be read"]
    known = {user.id for user in await hass.auth.async_get_users()}
    for row in users:
        if not isinstance(row, dict) or not isinstance(row.get("user_id"), str):
            continue
        user_id = row["user_id"]
        if user_id not in known:
            warnings.append("one saved user theme was skipped because that user is gone")
            continue
        try:
            store = await async_user_store(hass, user_id)
            await store.async_set_item("theme", copy.deepcopy(row.get("theme")))
        except Exception as err:  # noqa: BLE001
            warnings.append(f"could not restore a saved theme for one user: {err}")
    return warnings


async def _restore_global(hass: HomeAssistant, recorded: Any) -> str | None:
    if not isinstance(recorded, dict):
        return "the recorded default theme could not be read"
    name = recorded.get("name")
    if not isinstance(name, str) or not name:
        name = DEFAULT_THEME
    dark = recorded.get("name_dark")
    payload: dict[str, str] = {"name": name, "name_dark": dark if isinstance(dark, str) and dark else _NO_DARK_THEME}
    try:
        await hass.services.async_call("frontend", "set_theme", payload, blocking=True)
    except Exception as err:  # noqa: BLE001
        return f"frontend.set_theme did not run: {err}"
    return None


async def _delete_undo(hass: HomeAssistant) -> None:
    path = undo_file(hass.config.config_dir)

    def _delete() -> None:
        if path.is_symlink() or not path.is_file():
            return
        path.unlink()

    await hass.async_add_executor_job(_delete)


async def async_undo(hass: HomeAssistant) -> dict[str, Any]:
    """Restore the theme recorded before the last apply, then drop that record."""
    document = await _read_undo(hass)
    if document is None:
        raise BindError("nothing to undo", 404)
    warnings: list[str] = []
    view = document.get("view")
    if isinstance(view, dict):
        await _restore_view(hass, view)
    warnings.extend(await _restore_users(hass, document.get("users")))
    note = await _restore_global(hass, document.get("global"))
    if note:
        warnings.append(note)
    if warnings:
        return {"restored": True, "theme": document.get("theme"), "warnings": warnings}
    await _delete_undo(hass)
    return {"restored": True, "theme": document.get("theme"), "warnings": []}
