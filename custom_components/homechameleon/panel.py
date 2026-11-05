"""Sidebar panel that serves the built editor."""

from __future__ import annotations

from pathlib import Path

from homeassistant.components import frontend
from homeassistant.components.http import StaticPathConfig
from homeassistant.core import HomeAssistant

from .const import PANEL_ICON, PANEL_PATH, PANEL_TITLE, STATIC_URL

PANEL_DIR = Path(__file__).parent / "panel"
_REGISTERED = "homechameleon_static_registered"
# The built editor is an HTML document. A custom panel module_url is imported
# as JavaScript, so pointing one at this file shows "Unable to load the panel
# source". The built-in iframe panel loads the document instead.
EDITOR_URL = f"{STATIC_URL}/index.html?panel=1"


async def async_register(hass: HomeAssistant) -> None:
    """Serve the editor and add the admin-only sidebar item."""
    if PANEL_DIR.is_dir() and not hass.data.get(_REGISTERED):
        await hass.http.async_register_static_paths(
            [StaticPathConfig(STATIC_URL, str(PANEL_DIR), False)]
        )
        hass.data[_REGISTERED] = True
    frontend.async_register_built_in_panel(
        hass,
        "iframe",
        sidebar_title=PANEL_TITLE,
        sidebar_icon=PANEL_ICON,
        frontend_url_path=PANEL_PATH,
        config={"url": EDITOR_URL},
        require_admin=True,
    )


def async_unregister(hass: HomeAssistant) -> None:
    """Remove the sidebar item. Static files stay registered."""
    frontend.async_remove_panel(hass, PANEL_PATH)
