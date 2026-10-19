# v0.1 QA

Headless run for the pre-release. No device lab. The low-end tablet pass was skipped.

## Matrix

`scripts/qa-v0.1.sh` (CI job `qa`) reads `colors.toml` from the Omarchy checkout pinned in `.github/workflows/omarchy.yml` (`v4.0.5`) and generates 20 synthetic 64×64 photos in the `hc-qa` binary. Each case runs Sheer, Bubbles, Material, and Starship Console, in light and dark, with WCAG AA.

That tag has 22 theme directories, not 23. Every directory has a `colors.toml`. Those 22 are the whole stock set at the pin.

| | Count |
| --- | --- |
| Omarchy themes | 22 |
| Photos | 20 |
| Styles × modes | 4 × 2 |
| Cases | 336 |
| WCAG AA failures | 0 |
| PyYAML failures | 0 |
| Native vs wasm mismatches | 0 |

Each case must pass the core's AA contrast report, parse with PyYAML, and have the display name as the only top-level YAML key (`modes` sits under that key). Native and wasm stdout are byte-identical. Release timing on this VM: about 7s native and 29s wasm.

The wallpaper toggle stays off, which is the default. Sheer still checks text on the translucent card against the solid background. Photos are generated patterns, not camera files.

## Editor

Playwright smoke, 2 tests on each of chromium, firefox, webkit, and a Pixel 5 viewport: 8 passed. Webkit is the Safari stand-in. The smoke covers zip export, the panel Apply control, a style change, a slider, and the sample preview.

## Pages

Not published. `.github/workflows/pages.yml` builds `web/` and deploys only from `workflow_dispatch`. This repo is `homechameleon/homechameleon`, so the project site is `https://homechameleon.github.io/homechameleon/` and the build base is `/homechameleon/`. A user-site root would require a repository named `homechameleon.github.io`.
