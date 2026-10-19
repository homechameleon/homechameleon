"""Config flow, auth, limits, writes, and golden YAML for HomeChameleon."""

from __future__ import annotations

import base64
import io
import json
from pathlib import Path

import aiohttp
import pytest

from PIL import Image

from custom_components.homechameleon.const import DOMAIN, ISSUE_THEMES_INCLUDE, THEMES_INCLUDE_LINES
from custom_components.homechameleon.engine import generate_json
from custom_components.homechameleon.files import PathError, background_path, theme_yaml_path
from homeassistant.components.frontend import DATA_PANELS
from homeassistant.helpers import issue_registry as ir

REPO = Path(__file__).resolve().parents[1]
TOKYO = (REPO / "omarchy/tests/fixtures/tokyo-night.colors.toml").read_text(encoding="utf-8")
GOLDEN = (REPO / "crates/homechameleon-core/tests/golden/fixture.txt").read_text(encoding="utf-8")
THEMES_INCLUDE = "frontend:\n  themes: !include_dir_merge_named themes\n"


def _png() -> bytes:
    image = Image.new("RGB", (8, 8), (20, 80, 140))
    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    return buffer.getvalue()


def _tokyo_body(**extra) -> dict:
    body = {
        "name": "Tokyo Night",
        "style": "sheer",
        "mode": "dark",
        "palette": {"format": "omarchy-colors-toml", "data": TOKYO},
        "background": {"enabled": False},
        "apply": True,
        "set_default": True,
    }
    body.update(extra)
    return body


def _golden_tokyo_yaml() -> str:
    section = GOLDEN.split("===== tokyo-sheer-dark =====\n", 1)[1]
    return section.split("--- css ---\n", 1)[0]


async def test_config_flow_is_single_instance(hass):
    """The flow confirms once and refuses a second copy."""
    result = await hass.config_entries.flow.async_init(DOMAIN, context={"source": "user"})
    assert result["type"] == "form"
    assert result["step_id"] == "user"
    result = await hass.config_entries.flow.async_configure(result["flow_id"], {})
    assert result["type"] == "create_entry"
    assert result["title"] == "HomeChameleon"

    again = await hass.config_entries.flow.async_init(DOMAIN, context={"source": "user"})
    assert again["type"] == "abort"
    assert again["reason"] == "single_instance_allowed"


async def test_panel_is_admin_only(hass, setup_hc):
    """The sidebar item is HomeChameleon and hidden from non-admins."""
    panel = hass.data[DATA_PANELS]["homechameleon"]
    assert panel.sidebar_title == "HomeChameleon"
    assert panel.require_admin is True
    assert "panel=1" in panel.config["_panel_custom"]["module_url"]


async def test_styles_require_auth_and_admin(hass, setup_hc, hass_client, hass_client_no_auth, hass_read_only_access_token):
    """Styles are visible to an admin and refused otherwise."""
    admin = await hass_client()
    response = await admin.get("/api/homechameleon/styles")
    assert response.status == 200
    payload = await response.json()
    assert [item["id"] for item in payload["styles"]] == ["sheer", "bubbles", "material", "starship"]

    anonymous = await hass_client_no_auth()
    assert (await anonymous.get("/api/homechameleon/styles")).status == 401

    reader = await hass_client(hass_read_only_access_token)
    assert (await reader.post("/api/homechameleon/generate", json=_tokyo_body())).status == 403


async def test_unknown_style_is_400(hass, setup_hc, hass_client):
    """An unknown style id is rejected before anything is written."""
    client = await hass_client()
    response = await client.post("/api/homechameleon/generate", json=_tokyo_body(style="nope"))
    assert response.status == 400
    body = await response.json()
    assert "unknown style" in body["error"]
    assert not Path(hass.config.path("themes", "tokyo-night.yaml")).is_file()


async def test_image_over_10mb_is_413(hass, setup_hc, hass_client):
    """Decoded upload size over 10 MB is HTTP 413."""
    client = await hass_client()
    huge = base64.b64encode(b"x" * (10 * 1024 * 1024 + 1)).decode("ascii")
    response = await client.post(
        "/api/homechameleon/generate",
        json={
            "name": "Desk",
            "style": "sheer",
            "images": [{"data": huge, "weight": 1}],
            "apply": False,
        },
    )
    assert response.status == 413


async def test_background_off_writes_no_www_file(hass, setup_hc, hass_client):
    """Images are for colors only when the background toggle is off."""
    client = await hass_client()
    response = await client.post(
        "/api/homechameleon/generate",
        json={
            "name": "Desk",
            "style": "sheer",
            "mode": "dark",
            "images": [
                {
                    "data": base64.b64encode(_png()).decode("ascii"),
                    "weight": 1,
                    "use_as_background": True,
                }
            ],
            "background": {"enabled": False},
            "apply": True,
            "set_default": False,
        },
    )
    assert response.status == 200
    payload = await response.json()
    assert payload["applied"] is True
    assert payload["files"] == ["themes/desk.yaml"]
    assert "www/" not in "".join(payload["files"])
    yaml_path = Path(hass.config.path("themes", "desk.yaml"))
    text = yaml_path.read_text(encoding="utf-8")
    assert "lovelace-background:" not in text
    assert "/local/" not in text
    assert not Path(hass.config.path("www", "homechameleon")).exists()


async def test_background_on_writes_wallpaper(hass, setup_hc, hass_client):
    """An enabled background is the only case that writes under www/."""
    client = await hass_client()
    response = await client.post(
        "/api/homechameleon/generate",
        json={
            "name": "Desk",
            "style": "sheer",
            "mode": "dark",
            "images": [
                {
                    "data": base64.b64encode(_png()).decode("ascii"),
                    "weight": 1,
                    "use_as_background": True,
                }
            ],
            "background": {"enabled": True, "blur": 0, "dim": 0, "max_edge": 256},
            "apply": True,
            "set_default": True,
            "dashboard": {"create": True, "url_path": "homechameleon", "kind": "sections"},
        },
    )
    assert response.status == 200
    payload = await response.json()
    assert payload["default_set"] is True
    assert any(path.startswith("www/homechameleon/desk/bg-") for path in payload["files"])
    assert any("Starter dashboards" in note for note in payload["warnings"])
    written = list(Path(hass.config.path("www", "homechameleon", "desk")).glob("bg-*"))
    assert len(written) == 1
    yaml_text = Path(hass.config.path("themes", "desk.yaml")).read_text(encoding="utf-8")
    assert "/local/homechameleon/desk/" in yaml_text
    calls = hass.data["homechameleon_service_calls"]
    assert ("frontend", "reload_themes", {}) in calls
    assert ("frontend", "set_theme", {"name": "Desk", "name_dark": "Desk"}) in calls


async def test_preview_writes_nothing(hass, setup_hc, hass_client):
    """Preview returns YAML and does not touch themes or www."""
    client = await hass_client()
    response = await client.post("/api/homechameleon/preview", json=_tokyo_body())
    assert response.status == 200
    payload = await response.json()
    assert payload["applied"] is False
    assert payload["default_set"] is False
    assert payload["files"] == []
    assert payload["yaml"] == _golden_tokyo_yaml()
    assert not Path(hass.config.path("themes", "tokyo-night.yaml")).is_file()
    assert not Path(hass.config.path("www", "homechameleon")).exists()
    assert hass.data["homechameleon_service_calls"] == []


async def test_generate_yaml_matches_core_golden(hass, setup_hc, hass_client):
    """The wasmtime core writes the same Tokyo Night YAML as the native golden file."""
    direct = json.loads(
        generate_json(
            json.dumps(
                {
                    "name": "Tokyo Night",
                    "style": "sheer",
                    "mode": "dark",
                    "palette": {"format": "omarchy-colors-toml", "data": TOKYO},
                }
            )
        )
    )
    assert direct["yaml"] == _golden_tokyo_yaml()

    client = await hass_client()
    response = await client.post("/api/homechameleon/generate", json=_tokyo_body(apply=True, set_default=False))
    assert response.status == 200
    payload = await response.json()
    assert payload["theme"] == "Tokyo Night"
    assert payload["contrast"]["pass"] is True
    assert "checks" in payload["contrast"]
    assert set(payload) == {"theme", "files", "palette", "contrast", "applied", "default_set", "warnings"}
    written = Path(hass.config.path("themes", "tokyo-night.yaml")).read_text(encoding="utf-8")
    assert written == _golden_tokyo_yaml()


async def test_delete_rejects_path_traversal(hass, setup_hc, hass_client):
    """A slug with dots or slashes never leaves the theme directories."""
    secret = Path(hass.config.path("configuration.yaml"))
    before = secret.read_text(encoding="utf-8")
    client = await hass_client()
    for slug in ("..", "../configuration", "..%2F..%2Fconfiguration.yaml", "desk/../../configuration"):
        response = await client.delete(f"/api/homechameleon/theme/{slug}")
        assert response.status in {400, 404}
        assert secret.read_text(encoding="utf-8") == before
    with pytest.raises(PathError):
        theme_yaml_path(hass.config.config_dir, "../configuration")
    with pytest.raises(PathError):
        background_path(hass.config.config_dir, "desk", "../bg-00000000.webp")


async def test_delete_removes_only_the_theme(hass, setup_hc, hass_client):
    """Delete removes the yaml and the wallpaper folder for that slug."""
    client = await hass_client()
    created = await client.post(
        "/api/homechameleon/apply",
        json={
            "name": "Desk",
            "yaml": "modes:\n  dark:\n    primary-color: \"#336699\"\n",
            "set_default": False,
            "background": {
                "enabled": True,
                "path": "desk/bg-aabbccdd.webp",
                "data": base64.b64encode(b"webp-bytes").decode("ascii"),
            },
        },
    )
    assert created.status == 200
    response = await client.delete("/api/homechameleon/theme/desk")
    assert response.status == 200
    assert not Path(hass.config.path("themes", "desk.yaml")).exists()
    assert not Path(hass.config.path("www", "homechameleon", "desk")).exists()


async def test_multipart_generate(hass, setup_hc, hass_client):
    """Multipart file parts are images. The JSON part carries the rest of the request."""
    client = await hass_client()
    request = {
        "name": "Desk",
        "style": "material",
        "mode": "dark",
        "background": {"enabled": False},
        "apply": True,
        "set_default": False,
        "images": [{"weight": 1, "use_as_background": False}],
    }
    form = aiohttp.FormData()
    form.add_field("request", json.dumps(request))
    form.add_field("images", _png(), filename="board.png", content_type="image/png")
    response = await client.post("/api/homechameleon/generate", data=form)
    assert response.status == 200
    payload = await response.json()
    assert payload["theme"] == "Desk"
    assert payload["applied"] is True
    assert not Path(hass.config.path("www", "homechameleon", "desk")).exists()


async def test_missing_themes_include_raises_a_repair(hass, setup_hc):
    """A missing themes include becomes a Repairs issue with the exact lines to add."""
    issue = ir.async_get(hass).async_get_issue(DOMAIN, ISSUE_THEMES_INCLUDE)
    assert issue is not None
    assert issue.translation_placeholders["lines"] == THEMES_INCLUDE_LINES
    assert issue.translation_placeholders["lines"] == "frontend:\n  themes: !include_dir_merge_named themes"

    path = Path(hass.config.path("configuration.yaml"))
    original = path.read_text(encoding="utf-8")
    path.write_text(THEMES_INCLUDE, encoding="utf-8")
    from custom_components.homechameleon.repairs import async_check_themes_include

    await async_check_themes_include(hass)
    assert ir.async_get(hass).async_get_issue(DOMAIN, ISSUE_THEMES_INCLUDE) is None
    assert path.read_text(encoding="utf-8") == THEMES_INCLUDE
    assert original != path.read_text(encoding="utf-8")


async def test_merge_named_uses_the_display_name(hass, hass_client):
    """include_dir_merge_named names the theme from the top-level key."""
    from homeassistant.components.frontend import DATA_THEMES, _async_setup_themes
    from homeassistant.config import async_hass_config_yaml
    from homeassistant.setup import async_setup_component
    from pytest_homeassistant_custom_component.common import MockConfigEntry

    Path(hass.config.path("configuration.yaml")).write_text(
        "frontend:\n  themes: !include_dir_merge_named themes\n",
        encoding="utf-8",
    )
    themes_dir = Path(hass.config.path("themes"))
    themes_dir.mkdir(exist_ok=True)
    for child in themes_dir.iterdir():
        if child.is_file():
            child.unlink()
    assert await async_setup_component(hass, "http", {})
    loaded = await async_hass_config_yaml(hass)
    await _async_setup_themes(hass, loaded["frontend"]["themes"])
    entry = MockConfigEntry(domain=DOMAIN, data={}, title="HomeChameleon")
    entry.add_to_hass(hass)
    assert await hass.config_entries.async_setup(entry.entry_id)
    await hass.async_block_till_done()

    client = await hass_client()
    response = await client.post(
        "/api/homechameleon/generate",
        json=_tokyo_body(name="Omarchy", apply=True, set_default=True),
    )
    assert response.status == 200
    payload = await response.json()
    assert payload["theme"] == "Omarchy"
    assert payload["default_set"] is True
    assert payload["files"] == ["themes/omarchy.yaml"]
    written = Path(hass.config.path("themes", "omarchy.yaml")).read_text(encoding="utf-8")
    assert "\nOmarchy:\n  modes:\n" in f"\n{written}"
    reloaded = await async_hass_config_yaml(hass)
    themes = reloaded["frontend"]["themes"]
    assert "Omarchy" in themes
    assert "modes" not in themes
    assert "modes" in themes["Omarchy"]
    assert "Omarchy" in hass.data[DATA_THEMES]
    assert "modes" not in hass.data[DATA_THEMES]


async def test_apply_rejects_a_wallpaper_reference_when_background_is_off(hass, setup_hc, hass_client):
    """The panel cannot point a theme at /local/ while the background is off."""
    client = await hass_client()
    response = await client.post(
        "/api/homechameleon/apply",
        json={
            "name": "Desk",
            "yaml": 'lovelace-background: "center / cover url(\\"/local/homechameleon/desk/bg.webp\\")"\n',
            "background": {"enabled": False},
        },
    )
    assert response.status == 400
    assert not Path(hass.config.path("themes", "desk.yaml")).is_file()
