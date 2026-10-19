"""HomeChameleon for Home Assistant."""

from __future__ import annotations

from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers import config_validation as cv
from homeassistant.helpers.typing import ConfigType

from .const import DOMAIN
from .panel import async_register, async_unregister
from .repairs import async_check_themes_include
from .views import async_register_views

CONFIG_SCHEMA = cv.config_entry_only_config_schema(DOMAIN)

_VIEWS = "homechameleon_views_registered"


async def async_setup(hass: HomeAssistant, config: ConfigType) -> bool:
    """Set up the domain. The config entry does the work."""
    return True


async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    """Register the API, the panel, and the themes-include check."""
    await async_check_themes_include(hass)
    if not hass.data.get(_VIEWS):
        async_register_views(hass)
        hass.data[_VIEWS] = True
    await async_register(hass)
    return True


async def async_unload_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    """Remove the sidebar panel."""
    async_unregister(hass)
    return True
