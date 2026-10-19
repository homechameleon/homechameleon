"""Repairs issue when the themes include is missing.

HomeChameleon never edits configuration.yaml.
"""

from __future__ import annotations

from pathlib import Path

from homeassistant.core import HomeAssistant
from homeassistant.helpers import issue_registry as ir

from .const import DOMAIN, ISSUE_THEMES_INCLUDE, THEMES_INCLUDE_LINES, THEMES_INCLUDE_MARKER


def themes_include_present(text: str) -> bool:
    """Return True when configuration.yaml already merges the themes folder."""
    return THEMES_INCLUDE_MARKER in text


async def async_check_themes_include(hass: HomeAssistant) -> None:
    """Raise or clear the Repairs issue for the one-time themes include."""
    path = Path(hass.config.path("configuration.yaml"))

    def _read() -> str:
        if not path.is_file():
            return ""
        return path.read_text(encoding="utf-8")

    text = await hass.async_add_executor_job(_read)
    if themes_include_present(text):
        ir.async_delete_issue(hass, DOMAIN, ISSUE_THEMES_INCLUDE)
        return
    ir.async_create_issue(
        hass,
        DOMAIN,
        ISSUE_THEMES_INCLUDE,
        is_fixable=False,
        severity=ir.IssueSeverity.WARNING,
        translation_key=ISSUE_THEMES_INCLUDE,
        translation_placeholders={"lines": THEMES_INCLUDE_LINES},
    )
