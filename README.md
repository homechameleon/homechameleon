# HomeChameleon

Turn your photos into complete Home Assistant themes, in the style you pick.

![HomeChameleon editor](docs/hero.png)

HomeChameleon is not affiliated with or endorsed by the Open Home Foundation or Home Assistant, and is not affiliated with ha-chameleon. ha-chameleon colors RGB lights. HomeChameleon builds dashboard themes.

## What works today

The editor in [`web/`](web/) runs in the browser. Drop up to eight photos, or import an Omarchy `colors.toml`, a Base16 scheme, a hex list, or `homechameleon-palette/1` JSON (a `?palette=` link works too). Pick Sheer, Bubbles, Material, or Starship Console, then Dark, Light, or Auto. A sample dashboard follows the CSS variables from the generated theme. Undo, redo, and blueprints stay in this browser.

Export writes a zip with `themes/<slug>.yaml` and an install readme. A wallpaper is included under `www/homechameleon/<slug>/` only when the background toggle is on. It is off by default. An optional card-mod snippet is included only when that extra is on.

Photos never leave the browser. The editor does not upload them and does not ask you to sign in.

[`homechameleon-core`](crates/homechameleon-core) is the generator the editor runs as WebAssembly.

- Extraction uses OKLab k-means for the neutral and Material Color Utilities (quantize and score) for the accent. Modes: normal, pastel, material, colorful, muted. A mood board is a weighted merge of up to eight images.
- Palettes: Omarchy `colors.toml`, base16, a role map, a hex list, and `homechameleon-palette/1` JSON.
- Light and dark roles. WCAG AA or AAA contrast is part of choosing each tone. Sheer also checks text on the card composited over the wallpaper (or over the solid background when the wallpaper is off).
- Styles: Sheer, Bubbles, Material, and Starship Console. Bubble Card and Mushroom variables are included. card-mod CSS is off unless you ask for it.
- The same input produces the same bytes on native and on wasm.

The [`omarchy/`](omarchy/) folder is a shell helper for Omarchy. `homechameleon-sync --export` writes `homechameleon-palette/1` JSON from a `colors.toml` file. Details are in [omarchy/README.md](omarchy/README.md).

A background is off unless you turn it on. When it is on, the theme points at `/local/homechameleon/…`. Home Assistant serves files under `/local/` without a login, so anyone who can reach the instance and guess the file name can download that wallpaper.

```bash
cargo test
cd web && npm ci && npm run build:wasm && npm run dev
```

The crate is not published. Theme YAML is emitted by this crate; it does not use `serde_yaml` or `serde_yml`.

## License

Released under the [MIT License](LICENSE). Copyright (c) 2026 HomeChameleon contributors.

The Starship Console sample uses [Antonio](web/public/fonts/Antonio-Bold.woff2), licensed under the SIL Open Font License. The license is in [web/public/fonts/OFL.txt](web/public/fonts/OFL.txt).
