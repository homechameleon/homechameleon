"""Sidebar panel that serves the built editor."""

from __future__ import annotations

from pathlib import Path

from homeassistant.components import frontend, panel_custom
from homeassistant.components.http import StaticPathConfig
from homeassistant.core import HomeAssistant

from .const import PANEL_ICON, PANEL_PATH, PANEL_TITLE, STATIC_URL

PANEL_DIR = Path(__file__).parent / "panel"
_REGISTERED = "homechameleon_static_registered"


async def async_register(hass: HomeAssistant) -> None:
    """Serve the editor and add the admin-only sidebar item."""
    if PANEL_DIR.is_dir() and not hass.data.get(_REGISTERED):
        await hass.http.async_register_static_paths(
            [StaticPathConfig(STATIC_URL, str(PANEL_DIR), False)]
        )
        hass.data[_REGISTERED] = True
    await panel_custom.async_register_panel(
        hass,
        frontend_url_path=PANEL_PATH,
        webcomponent_name="homechameleon-panel",
        sidebar_title=PANEL_TITLE,
        sidebar_icon=PANEL_ICON,
        module_url=f"{STATIC_URL}/index.html?panel=1",
        embed_iframe=True,
        require_admin=True,
    )


def async_unregister(hass: HomeAssistant) -> None:
    """Remove the sidebar item. Static files stay registered."""
    frontend.async_remove_panel(hass, PANEL_PATH)
