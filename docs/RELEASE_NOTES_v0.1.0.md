# HomeChameleon v0.1.0

Draft release notes. Do not publish this file as a GitHub release, and do not create the `v0.1.0` tag, until the release pull request is merged.

Turn a wallpaper or a photo into a Home Assistant theme, edit it live, and apply it in one click.

## Highlights

- Palette from a wallpaper or a photo, or from an Omarchy `colors.toml`, a Base16 scheme, a hex list, or `homechameleon-palette/1` JSON.
- Live editing with a card preview.
- One-click apply from the Home Assistant sidebar panel, plus a zip export from the standalone editor.
- Styles: Bubbles, Sheer, Material, Starship Console, and Flat.
- Dark and light (and auto, which keeps both).

Screenshots from the local editor, using the Tokyo Night palette, are in [`docs/screenshots/`](screenshots/).

## Install with HACS

1. In HACS, add this repository as a custom repository (category: Integration): `https://github.com/homechameleon/homechameleon`
2. Install HomeChameleon and restart Home Assistant.
3. If these lines are not already in `configuration.yaml`, add them and restart again. HomeChameleon does not edit that file itself.

```yaml
frontend:
  themes: !include_dir_merge_named themes
```

4. Go to Settings, then Devices & services, and add HomeChameleon. Open the HomeChameleon sidebar item. That page is limited to administrators.

## Install for Omarchy

The AUR package is not published with this draft. From a checkout of the repository, after the integration is installed:

```bash
omarchy/bin/homechameleon-setup
```

Setup asks for the Home Assistant URL, an administrator long-lived access token, a style, and whether to send the desktop background (off by default). See [`omarchy/README.md`](../omarchy/README.md).

## Also in 0.1.0

- Optional Home Assistant add-on for installations that do not use HACS.
- Per-view apply, undo, and dashboard snapshots.
- Room views, previewed in light and dark before anything is written.
- `homechameleon-core` generates the same bytes on native and on WebAssembly.

HomeChameleon is not affiliated with or endorsed by the Open Home Foundation or Home Assistant, and is not affiliated with ha-chameleon.
