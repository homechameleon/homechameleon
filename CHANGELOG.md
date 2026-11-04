# Changelog

## 0.1.0

First release of HomeChameleon.

### Editor

- Browser editor that builds a palette from a wallpaper or a photo, or from an Omarchy `colors.toml`, a Base16 scheme, a hex list, or `homechameleon-palette/1` JSON.
- Live card preview, undo and redo, and blueprints stored in the browser.
- Dark, light, and auto. Export a zip of the theme. Photos stay in the browser unless a Home Assistant client posts one.

### Styles

- Sheer, Bubbles, Material, Flat, and Starship Console, each with its own shape, type, and optional card-mod CSS.
- Bubble Card and Mushroom variables are included. card-mod CSS stays off unless you ask for it.
- WCAG AA or AAA contrast is part of choosing each tone.

### Home Assistant

- Custom integration for HACS: sidebar panel, admin-only generate, preview, and apply API.
- One-click apply from the panel. Apply can target one dashboard view. Undo theme restores the previous themes. The default-theme checkbox starts off.
- Room tab: a Lovelace view for an area, previewed in light and dark before anything is written.
- Optional add-on for installations that do not use HACS. It uses the Supervisor token and does not edit `configuration.yaml`.

### Omarchy

- `homechameleon-sync` sends the current Omarchy palette to Home Assistant when the theme changes.
- `homechameleon-setup` writes a private config, installs the theme-set hook, and can install the palette template and a user path unit. Background sync is off by default.
- The AUR package is not part of this tag. See the ship checklist in the v0.1.0 pull request.

### This release prep

- Sample preview: Sheer entity names use the same light ink and shadow as the card titles, so they stay readable on the frosted pane.
- Sample preview: Starship Console keeps light text on its black console in light mode.
