"""Constants for the HomeChameleon integration."""

DOMAIN = "homechameleon"
PANEL_PATH = "homechameleon"
PANEL_TITLE = "HomeChameleon"
PANEL_ICON = "mdi:palette"
STATIC_URL = "/homechameleon_static"
ISSUE_THEMES_INCLUDE = "themes_include"

MAX_IMAGE_BYTES = 10 * 1024 * 1024
MAX_IMAGES = 8
MAX_YAML_BYTES = 1_000_000

THEMES_INCLUDE_LINES = "frontend:\n  themes: !include_dir_merge_named themes"
THEMES_INCLUDE_MARKER = "!include_dir_merge_named themes"

SLUG_RE_SOURCE = r"^[a-z0-9_-]+$"
BACKGROUND_FILE_RE_SOURCE = r"bg-[0-9a-f]{8}\.(?:webp|jpg)$"
