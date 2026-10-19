# HomeChameleon

Turn your photos into complete Home Assistant themes, in the style you pick.

HomeChameleon is not affiliated with or endorsed by the Open Home Foundation or Home Assistant, and is not affiliated with ha-chameleon. ha-chameleon colors RGB lights. HomeChameleon builds dashboard themes.

## What works today

[`homechameleon-core`](crates/homechameleon-core) reads raw RGBA pixels or a palette and writes one Home Assistant theme file, plus a preview CSS map that uses the same variables.

- Extraction uses OKLab k-means for the neutral and Material Color Utilities (quantize and score) for the accent. Modes: normal, pastel, material, colorful, muted. A mood board is a weighted merge of up to eight images.
- Palettes: Omarchy `colors.toml`, base16, a role map, a hex list, and `homechameleon-palette/1` JSON.
- Light and dark roles. WCAG AA or AAA contrast is part of choosing each tone. Sheer also checks text on the card composited over the wallpaper (or over the solid background when the wallpaper is off).
- Styles: Sheer, Bubbles, Material, and Starship Console. Bubble Card and Mushroom variables are included. card-mod CSS is off unless you ask for it.
- The same input produces the same bytes on native and on wasm.

The [`omarchy/`](omarchy/) folder is a shell helper for Omarchy. `homechameleon-sync --export` writes `homechameleon-palette/1` JSON from a `colors.toml` file. Details are in [omarchy/README.md](omarchy/README.md).

A background is off unless you turn it on. When it is on, the theme points at `/local/homechameleon/…`. Home Assistant serves files under `/local/` without a login, so anyone who can reach the instance and guess the file name can download that wallpaper.

```bash
cargo test
```

The crate is not published. Theme YAML is emitted by this crate; it does not use `serde_yaml` or `serde_yml`.

## License

Released under the [MIT License](LICENSE). Copyright (c) 2026 HomeChameleon contributors.
