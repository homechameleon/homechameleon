"""Per-view theme assignment, undo, and dashboard snapshots."""

from __future__ import annotations

from pathlib import Path

import pytest

from custom_components.homechameleon.dashboards import BindError
from custom_components.homechameleon.files import PathError, snapshot_file, theme_yaml_path
from homeassistant.components.frontend import DATA_DEFAULT_DARK_THEME, DATA_DEFAULT_THEME
from homeassistant.components.frontend.storage import async_user_store
from homeassistant.components.lovelace import LovelaceData
from homeassistant.components.lovelace.const import LOVELACE_DATA
from homeassistant.components.lovelace.dashboard import LovelaceStorage, LovelaceYAML

YAML = 'Desk:\n  modes:\n    dark:\n      primary-color: "#336699"\n'
VIEWS = [
    {
        "title": "Home",
        "path": "home",
        "theme": "noctis",
        "cards": [{"type": "markdown", "content": "keep"}],
    },
    {"title": "Night", "cards": [{"type": "markdown", "content": "stay"}]},
]


async def _storage_board(hass, views=None, *, url_path="overview-board", title="Rooms"):
    """Register one storage dashboard without starting the whole Lovelace integration."""
    board = LovelaceStorage(
        hass,
        {
            "id": url_path,
            "url_path": url_path,
            "title": title,
            "require_admin": False,
            "show_in_sidebar": True,
            "mode": "storage",
        },
    )
    await board.async_save({"views": views if views is not None else VIEWS})
    hass.data[LOVELACE_DATA] = LovelaceData(
        resource_mode="storage",
        dashboards={url_path: board},
        resources=None,
        yaml_dashboards={},
    )
    return board


def _apply_body(**extra):
    body = {
        "name": "Desk",
        "yaml": YAML,
        "background": {"enabled": False},
        "view": {"url_path": "overview-board", "path": "home"},
    }
    body.update(extra)
    return body


async def test_view_apply_records_themes_and_undoes_in_one_step(hass, setup_hc, hass_client):
    """Apply sets one view, keeps set_default off, and undo restores every recorded theme."""
    secret = Path(hass.config.path("configuration.yaml"))
    before = secret.read_text(encoding="utf-8")
    board = await _storage_board(hass)
    hass.data[DATA_DEFAULT_THEME] = "paper"
    hass.data[DATA_DEFAULT_DARK_THEME] = "ink"
    client = await hass_client()
    user = await hass.auth.async_create_user("Casey")
    store = await async_user_store(hass, user.id)
    await store.async_set_item("theme", {"theme": "paper", "dark": True})

    response = await client.post("/api/homechameleon/apply", json=_apply_body())
    assert response.status == 200
    payload = await response.json()
    assert payload["applied"] is True
    assert payload["default_set"] is False
    assert payload["undo"] is True
    assert payload["view"]["path"] == "home"
    assert payload["view"]["theme"] == "Desk"
    assert payload["snapshot"]

    config = await board.async_load(False)
    assert config["views"][0]["theme"] == "Desk"
    assert config["views"][0]["cards"] == [{"type": "markdown", "content": "keep"}]
    assert "theme" not in config["views"][1]
    assert config["views"][1]["cards"] == [{"type": "markdown", "content": "stay"}]

    calls = hass.data["homechameleon_service_calls"]
    assert ("frontend", "reload_themes", {}) in calls
    assert not any(call[1] == "set_theme" for call in calls)
    assert secret.read_text(encoding="utf-8") == before

    listed = await client.get("/api/homechameleon/snapshots")
    summaries = (await listed.json())["snapshots"]
    assert summaries[0]["id"] == payload["snapshot"]
    assert summaries[0]["reason"] == "before_view_theme"
    assert summaries[0]["url_path"] == "overview-board"
    document = await (await client.get(f"/api/homechameleon/snapshots/{payload['snapshot']}")).json()
    assert document["config"]["views"][0]["theme"] == "noctis"
    assert document["config"]["views"][0]["cards"][0]["content"] == "keep"
    undo_file = Path(hass.config.path("www", "homechameleon", "undo", "theme.json"))
    assert undo_file.is_file()
    assert undo_file.resolve().is_relative_to(Path(hass.config.path("www", "homechameleon")).resolve())

    status = await (await client.get("/api/homechameleon/undo")).json()
    assert status["available"] is True
    assert status["theme"] == "Desk"
    assert status["view"]["path"] == "home"
    assert "users" not in status

    await store.async_set_item("theme", {"theme": "other", "dark": False})
    hass.data[DATA_DEFAULT_THEME] = "changed"
    hass.data[DATA_DEFAULT_DARK_THEME] = "changed-dark"
    mutated = await board.async_load(False)
    mutated["views"][0]["theme"] = "other"
    await board.async_save(mutated)

    undone = await client.post("/api/homechameleon/undo")
    assert undone.status == 200
    body = await undone.json()
    assert body["restored"] is True
    assert body["warnings"] == []
    restored = await board.async_load(False)
    assert restored["views"][0]["theme"] == "noctis"
    assert restored["views"][0]["cards"][0]["content"] == "keep"
    assert "theme" not in restored["views"][1]
    assert store.data["theme"] == {"theme": "paper", "dark": True}
    assert ("frontend", "set_theme", {"name": "paper", "name_dark": "ink"}) in hass.data["homechameleon_service_calls"]
    assert not undo_file.exists()
    assert (await client.post("/api/homechameleon/undo")).status == 404
    assert Path(hass.config.path("themes", "desk.yaml")).is_file()
    assert secret.read_text(encoding="utf-8") == before

    indexed = await client.post(
        "/api/homechameleon/apply",
        json=_apply_body(view={"url_path": "overview-board", "index": 1}),
    )
    assert indexed.status == 200
    indexed_config = await board.async_load(False)
    assert indexed_config["views"][0]["theme"] == "noctis"
    assert indexed_config["views"][1]["theme"] == "Desk"
    assert (await client.post("/api/homechameleon/undo")).status == 200
    after_index = await board.async_load(False)
    assert "theme" not in after_index["views"][1]
    assert after_index["views"][1]["cards"][0]["content"] == "stay"


async def test_snapshot_list_restore_and_delete(hass, setup_hc, hass_client):
    """A snapshot is a full copy. Restore puts it back, and delete removes only that file."""
    secret = Path(hass.config.path("configuration.yaml"))
    before = secret.read_text(encoding="utf-8")
    board = await _storage_board(hass)
    client = await hass_client()
    created = await client.post("/api/homechameleon/snapshots", json={"url_path": "overview-board"})
    assert created.status == 200
    summary = await created.json()
    assert summary["reason"] == "manual"
    assert summary["views"] == 2
    snapshot_id = summary["id"]

    await board.async_save({"views": [{"title": "Home", "path": "home", "theme": "other", "cards": []}]})
    restored = await client.post(f"/api/homechameleon/snapshots/{snapshot_id}/restore")
    assert restored.status == 200
    assert (await restored.json())["restored"] is True
    config = await board.async_load(False)
    assert config["views"][0]["theme"] == "noctis"
    assert config["views"][0]["cards"][0]["content"] == "keep"
    assert len(config["views"]) == 2

    removed = await client.delete(f"/api/homechameleon/snapshots/{snapshot_id}")
    assert removed.status == 200
    assert (await client.get("/api/homechameleon/snapshots")).status == 200
    assert (await (await client.get("/api/homechameleon/snapshots")).json())["snapshots"] == []
    assert (await client.delete(f"/api/homechameleon/snapshots/{snapshot_id}")).status == 404
    assert secret.read_text(encoding="utf-8") == before
    assert not Path(hass.config.path("themes")).joinpath("overview-board.yaml").exists()


async def test_yaml_dashboard_is_copied_but_not_edited(hass, setup_hc, hass_client):
    """YAML layouts can be snapshotted. Apply and restore refuse to write the file."""
    filename = "room-board.yaml"
    path = Path(hass.config.path(filename))
    path.write_text(
        "views:\n  - title: Home\n    path: home\n    theme: noctis\n",
        encoding="utf-8",
    )
    original = path.read_text(encoding="utf-8")
    secret = Path(hass.config.path("configuration.yaml"))
    secret_before = secret.read_text(encoding="utf-8")
    board = LovelaceYAML(
        hass,
        "yaml-board",
        {
            "id": "yaml-board",
            "url_path": "yaml-board",
            "title": "YAML rooms",
            "filename": filename,
            "mode": "yaml",
            "require_admin": False,
            "show_in_sidebar": True,
        },
    )
    hass.data[LOVELACE_DATA] = LovelaceData(
        resource_mode="yaml",
        dashboards={"yaml-board": board},
        resources=None,
        yaml_dashboards={"yaml-board": {}},
    )
    client = await hass_client()
    listed = await (await client.get("/api/homechameleon/dashboards")).json()
    assert listed["dashboards"][0]["editable"] is False
    assert listed["dashboards"][0]["views"][0]["theme"] == "noctis"

    applied = await client.post(
        "/api/homechameleon/apply",
        json=_apply_body(view={"url_path": "yaml-board", "path": "home"}),
    )
    assert applied.status == 400
    assert "does not edit that file" in (await applied.json())["error"]
    assert path.read_text(encoding="utf-8") == original
    assert not Path(hass.config.path("themes", "desk.yaml")).exists()
    assert (await (await client.get("/api/homechameleon/snapshots")).json())["snapshots"] == []

    created = await client.post("/api/homechameleon/snapshots", json={"url_path": "yaml-board"})
    assert created.status == 200
    snapshot_id = (await created.json())["id"]
    restored = await client.post(f"/api/homechameleon/snapshots/{snapshot_id}/restore")
    assert restored.status == 400
    assert "does not edit that file" in (await restored.json())["error"]
    assert path.read_text(encoding="utf-8") == original
    assert secret.read_text(encoding="utf-8") == secret_before


async def test_failed_snapshot_leaves_the_view_unchanged(hass, setup_hc, hass_client, monkeypatch):
    """A view theme is not written when the required snapshot cannot be stored."""
    board = await _storage_board(hass)
    client = await hass_client()

    async def _fail(*_args, **_kwargs):
        raise BindError("dashboard snapshot is too large", 400)

    monkeypatch.setattr("custom_components.homechameleon.dashboards._write_snapshot", _fail)
    response = await client.post("/api/homechameleon/apply", json=_apply_body())
    assert response.status == 400
    config = await board.async_load(False)
    assert config["views"][0]["theme"] == "noctis"
    assert not Path(hass.config.path("www", "homechameleon", "undo", "theme.json")).exists()


async def test_missing_view_writes_nothing(hass, setup_hc, hass_client):
    """An unknown dashboard or view is rejected before the theme file is written."""
    await _storage_board(hass)
    client = await hass_client()
    missing_board = await client.post(
        "/api/homechameleon/apply",
        json=_apply_body(view={"url_path": "no-such-board", "path": "home"}),
    )
    assert missing_board.status == 404
    missing_view = await client.post(
        "/api/homechameleon/apply",
        json=_apply_body(view={"url_path": "overview-board", "path": "attic"}),
    )
    assert missing_view.status == 404
    bad_shape = await client.post("/api/homechameleon/apply", json=_apply_body(view={"url_path": "overview-board"}))
    assert bad_shape.status == 400
    assert not Path(hass.config.path("themes", "desk.yaml")).exists()
    plain = await client.post("/api/homechameleon/apply", json={"name": "Desk", "yaml": YAML})
    assert plain.status == 200
    payload = await plain.json()
    assert payload["default_set"] is False
    assert "undo" not in payload
    assert "snapshot" not in payload
    assert (await (await client.get("/api/homechameleon/undo")).json())["available"] is False
    assert (await (await client.get("/api/homechameleon/snapshots")).json())["snapshots"] == []
    assert not any(call[1] == "set_theme" for call in hass.data["homechameleon_service_calls"])


async def test_set_default_undo_restores_the_previous_global_theme(hass, setup_hc, hass_client):
    """Turning on the default still records the previous global theme for undo."""
    await _storage_board(hass)
    hass.data[DATA_DEFAULT_THEME] = "paper"
    hass.data[DATA_DEFAULT_DARK_THEME] = None
    client = await hass_client()
    response = await client.post(
        "/api/homechameleon/apply",
        json=_apply_body(set_default=True),
    )
    assert response.status == 200
    payload = await response.json()
    assert payload["default_set"] is True
    assert payload["undo"] is True
    calls = hass.data["homechameleon_service_calls"]
    assert ("frontend", "set_theme", {"name": "Desk", "name_dark": "Desk"}) in calls
    undone = await client.post("/api/homechameleon/undo")
    assert undone.status == 200
    assert (
        "frontend",
        "set_theme",
        {"name": "paper", "name_dark": "none"},
    ) in hass.data["homechameleon_service_calls"]
    config = await (await _board(hass)).async_load(False)
    assert config["views"][0]["theme"] == "noctis"


async def _board(hass):
    return hass.data[LOVELACE_DATA].dashboards["overview-board"]


async def test_generate_can_target_a_view(hass, setup_hc, hass_client):
    """Generate uses the same view assignment as apply, with set_default still off."""
    board = await _storage_board(hass)
    client = await hass_client()
    from tests.test_homechameleon import _tokyo_body

    response = await client.post(
        "/api/homechameleon/generate",
        json=_tokyo_body(
            apply=True,
            set_default=False,
            view={"url_path": "overview-board", "path": "home"},
        ),
    )
    assert response.status == 200
    payload = await response.json()
    assert payload["default_set"] is False
    assert payload["undo"] is True
    assert payload["theme"] == "Tokyo Night"
    config = await board.async_load(False)
    assert config["views"][0]["theme"] == "Tokyo Night"
    assert not any(call[1] == "set_theme" for call in hass.data["homechameleon_service_calls"])


async def test_theme_endpoints_require_an_admin(
    hass, setup_hc, hass_client, hass_client_no_auth, hass_read_only_access_token
):
    """Snapshot and undo routes are admin-only."""
    await _storage_board(hass)
    admin = await hass_client()
    assert (await admin.get("/api/homechameleon/dashboards")).status == 200

    anonymous = await hass_client_no_auth()
    assert (await anonymous.get("/api/homechameleon/dashboards")).status == 401
    assert (await anonymous.get("/api/homechameleon/snapshots")).status == 401
    assert (await anonymous.post("/api/homechameleon/undo")).status == 401

    reader = await hass_client(hass_read_only_access_token)
    assert (await reader.get("/api/homechameleon/snapshots")).status == 403
    assert (await reader.post("/api/homechameleon/snapshots", json={"url_path": "overview-board"})).status == 403
    assert (await reader.post("/api/homechameleon/undo")).status == 403
    assert (await reader.get("/api/homechameleon/undo")).status == 403


def test_snapshot_paths_and_reserved_theme_names_stay_inside_www(tmp_path):
    """Snapshot ids cannot leave www/homechameleon, and theme slugs cannot reuse those folders."""
    dest = snapshot_file(str(tmp_path), "overview-board-1")
    assert dest == tmp_path / "www" / "homechameleon" / "snapshots" / "overview-board-1.json"
    for snapshot_id in ("../configuration", "..", "a/b", "snap.json", "a" * 81):
        with pytest.raises(PathError):
            snapshot_file(str(tmp_path), snapshot_id)
    for slug in ("snapshots", "undo", "../configuration"):
        with pytest.raises(PathError):
            theme_yaml_path(str(tmp_path), slug)


async def test_reserved_theme_name_is_rejected(hass, setup_hc, hass_client):
    """A theme named Snapshots must not delete or replace snapshot storage."""
    client = await hass_client()
    response = await client.post(
        "/api/homechameleon/apply",
        json={"name": "Snapshots", "yaml": 'Snapshots:\n  modes:\n    dark:\n      primary-color: "#fff"\n'},
    )
    assert response.status == 400
    assert "reserved" in (await response.json())["error"]
    assert not Path(hass.config.path("themes", "snapshots.yaml")).exists()
