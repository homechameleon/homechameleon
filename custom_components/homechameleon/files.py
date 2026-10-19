"""Atomic writes confined to themes/ and www/homechameleon/."""

from __future__ import annotations

import os
import re
import shutil
import tempfile
from pathlib import Path

from homeassistant.core import HomeAssistant

from .const import BACKGROUND_FILE_RE_SOURCE, SLUG_RE_SOURCE

SLUG_RE = re.compile(SLUG_RE_SOURCE)
_BG_FILE_RE = re.compile(BACKGROUND_FILE_RE_SOURCE)


class PathError(ValueError):
    """A slug or path would leave the allowed directories."""


def theme_yaml_path(config_dir: str, slug: str) -> Path:
    """Return themes/<slug>.yaml, or raise if the slug is not safe."""
    if not SLUG_RE.fullmatch(slug):
        raise PathError("theme name is not a safe file name")
    root = Path(config_dir, "themes").resolve()
    dest = (root / f"{slug}.yaml").resolve()
    if dest.parent != root:
        raise PathError("theme name is not a safe file name")
    return dest


def background_path(config_dir: str, slug: str, relative: str) -> Path:
    """Return www/homechameleon/<slug>/<file>, or raise."""
    if not SLUG_RE.fullmatch(slug):
        raise PathError("theme name is not a safe file name")
    name = Path(relative).name
    parent = Path(relative).parent.as_posix()
    if parent not in {slug, "."} or not _BG_FILE_RE.fullmatch(name):
        raise PathError("background file name is not allowed")
    root = Path(config_dir, "www", "homechameleon").resolve()
    folder = (root / slug).resolve()
    dest = (folder / name).resolve()
    if folder.parent != root or dest.parent != folder:
        raise PathError("background file name is not allowed")
    return dest


def theme_dir_path(config_dir: str, slug: str) -> Path:
    """Return www/homechameleon/<slug>, or raise."""
    if not SLUG_RE.fullmatch(slug):
        raise PathError("theme name is not a safe file name")
    root = Path(config_dir, "www", "homechameleon").resolve()
    dest = (root / slug).resolve()
    if dest.parent != root:
        raise PathError("theme name is not a safe file name")
    return dest


def atomic_write(path: Path, data: bytes) -> None:
    """Write via a temp file in the same directory, then replace."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(prefix=".homechameleon-", suffix=".tmp", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp_name, path)
    except Exception:
        try:
            os.unlink(tmp_name)
        except OSError:
            pass
        raise


def delete_theme_files(config_dir: str, slug: str) -> bool:
    """Remove the theme YAML and its wallpaper folder. Returns whether anything was there."""
    yaml_path = theme_yaml_path(config_dir, slug)
    asset_dir = theme_dir_path(config_dir, slug)
    found = False
    if yaml_path.is_file():
        yaml_path.unlink()
        found = True
    if asset_dir.is_dir() and not asset_dir.is_symlink():
        shutil.rmtree(asset_dir)
        found = True
    return found


async def async_reload_themes(hass: HomeAssistant) -> str | None:
    """Call frontend.reload_themes. Returns a warning when the call fails."""
    try:
        await hass.services.async_call("frontend", "reload_themes", {}, blocking=True)
    except Exception as err:  # noqa: BLE001 - surface a short warning, keep the file
        return f"frontend.reload_themes did not run: {err}"
    return None


async def async_set_default_theme(hass: HomeAssistant, slug: str) -> str | None:
    """Call frontend.set_theme for the light and dark names."""
    try:
        await hass.services.async_call(
            "frontend",
            "set_theme",
            {"name": slug, "name_dark": slug},
            blocking=True,
        )
    except Exception as err:  # noqa: BLE001
        return f"frontend.set_theme did not run: {err}"
    return None
