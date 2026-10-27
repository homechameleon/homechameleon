# HomeChameleon for Omarchy

Your Home Assistant dashboard follows your Omarchy theme. Every time you switch
themes in Omarchy, `homechameleon-sync` sends the new palette to the
HomeChameleon integration in Home Assistant, which builds a theme in your chosen
style and applies it.

> [!IMPORTANT]
> **Unofficial community project.** HomeChameleon for Omarchy is not affiliated
> with or endorsed by Omarchy, Basecamp, the Open Home Foundation or Home
> Assistant, and it is **not affiliated with ha-chameleon** (a separate project
> that colours RGB lights). "Home Assistant" is a trademark of the Open Home
> Foundation and is used only to describe compatibility.

Status: MVP draft. No repository, package or release exists yet. Licence: MIT (see `../LICENSE`).

This code lives in the `omarchy/` folder of the HomeChameleon monorepo
(https://github.com/homechameleon/homechameleon). Its releases are tagged
`omarchy-vX.Y.Z` (for example `omarchy-v0.1.0`), separately from the other parts.

## Requirements

- Omarchy v4 (reads `~/.local/state/omarchy/current/theme/colors.toml`)
- The HomeChameleon integration installed in Home Assistant (provides
  `POST /api/homechameleon/generate`)
- A long-lived access token from a Home Assistant **administrator**
  (your profile > Security > Long-lived access tokens)
- `curl` and `jq`. For background sync: ImageMagick, plus ffmpeg for video
  backgrounds. Omarchy ships all four.

## Install

From the AUR (planned): `homechameleon-omarchy`, then run:

```bash
homechameleon-setup
```

Setup asks for your Home Assistant URL, the token (input hidden), a style id and
whether to send your background (default: **no**). It then:

1. writes `~/.config/homechameleon/config` with mode 600 (it holds the token),
2. installs the theme-set hook with `omarchy hook install theme-set`
   (a two-line wrapper in `~/.config/omarchy/hooks/theme-set.d/`),
3. optionally installs the palette template
   `~/.config/omarchy/themed/homechameleon.json.tpl`, which Omarchy renders on every
   theme switch,
4. optionally installs a systemd user path unit, so changing only the background
   (`omarchy theme bg next`) also syncs,
5. optionally sends a preview request to test the connection. A preview writes nothing.

Any file it would change is backed up first to
`~/.local/state/homechameleon/backups/<timestamp>/`.

In Home Assistant, set your profile theme to the default theme, so the synced
theme is the one you see.

## Usage

```bash
homechameleon-sync              # send the current palette (what the hook runs)
homechameleon-sync --preview    # ask Home Assistant for a preview; writes nothing
homechameleon-sync --styles     # list the styles your integration offers
homechameleon-sync --dry-run    # print the request body (image data shortened)
homechameleon-sync --export     # write ~/Downloads/homechameleon-palette-<theme>.json
homechameleon-sync --export DIR --with-wallpaper
```

If Home Assistant answers 413, the background image was over the 10 MB limit:
lower `MAX_EDGE` or set `BACKGROUND=0`.

The hook never breaks a theme switch. Errors are logged to
`~/.local/state/homechameleon/sync.log` and the hook exits 0. Use `--strict` for a
non-zero exit status on failure. Warnings from Home Assistant are shown and sent
as desktop notifications when `notify-send` is available.

## Config

`~/.config/homechameleon/config`, plain `KEY=value` lines. The file is parsed,
never executed.

| Key | Default | Meaning |
|---|---|---|
| `URL` | | Home Assistant base URL, e.g. `http://localhost:8123` |
| `TOKEN` | | Long-lived access token (administrator) |
| `NAME` | `Omarchy` | Theme name in Home Assistant. `@theme` uses the Omarchy theme's name instead (for example "Tokyo Night"), so each Omarchy theme gets its own Home Assistant theme |
| `STYLE` | `sheer` | Style id, passed through as is. Launch styles: `sheer`, `bubbles`, `material`, `flat`, `starship` |
| `BACKGROUND` | `0` | `1` = also send the desktop background (see Privacy) |
| `BG_BLUR`, `BG_DIM` | `0`, `0.0` | Background processing hints for the integration |
| `MAX_EDGE` | `2560` | Longest edge of the WebP sent (64-2560) |
| `BUBBLE`, `MUSHROOM`, `CARD_MOD` | `1`, `1`, `0` | Extra variables for Bubble Card, Mushroom and card-mod |
| `CONTRAST` | `AA` | Contrast target |
| `APPLY`, `SET_DEFAULT` | `1`, `1` | Write and reload the theme; make it the default theme |
| `TIMEOUT` | `30` | HTTP timeout in seconds |

## Privacy

**Background sync is opt-in and off by default.** By default only your colour
palette (the text of `colors.toml`) is sent.

If you turn it on (`BACKGROUND=1`), your current desktop background is downscaled,
converted to WebP and sent to Home Assistant. HomeChameleon stores it in the
`www` folder of your Home Assistant config. **Home Assistant serves files under
`/local/` without a login**, so anyone who can reach your Home Assistant instance
and knows or guesses the file name can download your wallpaper. If your
instance is reachable from the internet, think twice before turning this on.

The token is stored only in your config file (mode 600). curl reads it from a
private temporary file, so it never shows up in the process list or in the
log.

## Palette export format

`--export` writes our own `homechameleon-palette/1` JSON, using Omarchy's key
names:

```json
{
  "schema": "homechameleon-palette/1",
  "source": "omarchy",
  "theme": "tokyo-night",
  "mode": "dark",
  "colors": { "accent": "#7aa2f7", "background": "#1a1b26", "foreground": "#a9b1d6", "color0": "#1a1b26", "color15": "#c0caf5" },
  "background_file": "homechameleon-tokyo-night-background.webp"
}
```

`theme` is the Omarchy theme name. `background_file` is only present when the
background was exported too (`BACKGROUND=1` or `--with-wallpaper`). `colors`
always has the same keys in the same order, including `color0`-`color15`,
whichever source is used: Omarchy's render of the palette template, then
`omarchy-theme-color --all`, then the flat `colors.toml` keys with the ANSI
colours derived in Omarchy's order (color0 background, 1-6 red, green, yellow,
blue, magenta, cyan, 7 foreground, 8 muted, 9-14 the bright colours, 15
bright_foreground). The tests check that all three give the same file.

## What is sent

`POST <URL>/api/homechameleon/generate` with `Authorization: Bearer <token>`:

```json
{
  "name": "Omarchy",
  "style": "sheer",
  "mode": "dark",
  "palette": { "format": "omarchy-colors-toml", "data": "<raw colors.toml text>" },
  "background": { "enabled": false, "blur": 0, "dim": 0.0, "max_edge": 2560 },
  "options": { "bubble": true, "mushroom": true, "card_mod": false, "contrast": "AA" },
  "apply": true,
  "set_default": true
}
```

With `BACKGROUND=1`, the body also has
`"images": [{"data": "<base64 WebP>", "weight": 1.0, "use_as_background": true}]`
and `background.enabled` is `true`. `mode` comes from the `mode` key in
`colors.toml` (falling back to a `light.mode` marker file, then `dark`).

## Uninstall

```bash
homechameleon-setup --uninstall
```

This removes the config, hook, template and units, backing each one up first.
The log and backups are kept. The theme itself stays in Home Assistant.

## Development

```bash
tests/run-tests.sh                                   # needs jq, curl, python3; shellcheck, ImageMagick, ffmpeg for the full set
OMARCHY_CLONE=/path/to/omarchy tests/run-tests.sh    # also uses Omarchy's own template renderer and hook runner
```

The tests run in a throwaway `HOME` against a local mock of the HomeChameleon API
(`tests/mock_server.py`).
