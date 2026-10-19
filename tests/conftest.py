"""Home Assistant fixtures for the HomeChameleon integration."""

import shutil
from pathlib import Path

import pytest

pytest_plugins = "pytest_homeassistant_custom_component"


@pytest.fixture(autouse=True)
def auto_enable_custom_integrations(enable_custom_integrations):
    """Load this repository's custom_components package."""
    return


@pytest.fixture
def configuration_text() -> str:
    """Default configuration.yaml contents for a test."""
    return "default_config:\n"


@pytest.fixture
async def setup_hc(hass, configuration_text):
    """Write configuration.yaml, register frontend services, and set up the entry."""
    from homeassistant.setup import async_setup_component
    from pytest_homeassistant_custom_component.common import MockConfigEntry

    from custom_components.homechameleon.const import DOMAIN

    assert await async_setup_component(hass, "http", {})

    path = Path(hass.config.path("configuration.yaml"))
    path.write_text(configuration_text, encoding="utf-8")
    themes = Path(hass.config.path("themes"))
    if themes.is_dir():
        for child in themes.iterdir():
            if child.is_file():
                child.unlink()
    www = Path(hass.config.path("www", "homechameleon"))
    if www.is_dir():
        shutil.rmtree(www)
    calls: list[tuple[str, str, dict]] = []

    async def _record(call):
        calls.append((call.domain, call.service, dict(call.data)))

    hass.services.async_register("frontend", "reload_themes", _record)
    hass.services.async_register("frontend", "set_theme", _record)
    hass.data["homechameleon_service_calls"] = calls

    entry = MockConfigEntry(domain=DOMAIN, data={}, title="HomeChameleon")
    entry.add_to_hass(hass)
    assert await hass.config_entries.async_setup(entry.entry_id)
    await hass.async_block_till_done()
    return entry
