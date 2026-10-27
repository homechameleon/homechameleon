#!/usr/bin/env bash
# v0.1 QA matrix: Omarchy themes plus 20 synthetic photos, five styles, light and dark.
# Native and wasm stdout must match. PyYAML must see one top-level key, the display name.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
ref=${OMARCHY_REF:-v4.0.5}
themes=${QA_THEMES:-}
if [[ -z "$themes" ]]; then
  themes=${RUNNER_TEMP:-/tmp}/omarchy-${ref}/themes
  if [[ ! -d "$themes" ]]; then
    git clone --depth 1 --branch "$ref" https://github.com/basecamp/omarchy "${themes%/themes}"
  fi
fi

native=${NATIVE_BIN:-$root/target/release/hc-qa}
wasm=${WASM_BIN:-$root/target/wasm32-wasip1/release/hc-qa.wasm}
if [[ ! -x "$native" || ! -f "$wasm" ]]; then
  cargo build --release -p homechameleon-core --bin hc-qa
  cargo build --release --target wasm32-wasip1 -p homechameleon-core --bin hc-qa
fi

if ! command -v wasmtime >/dev/null 2>&1; then
  if [[ -x "$HOME/.wasmtime/bin/wasmtime" ]]; then
    export PATH="$HOME/.wasmtime/bin:$PATH"
  else
    curl -fsSL https://wasmtime.dev/install.sh | bash -s -- --version v49.0.3
    export PATH="$HOME/.wasmtime/bin:$PATH"
  fi
fi

out=${QA_OUT:-${RUNNER_TEMP:-/tmp}/homechameleon-qa}
mkdir -p "$out"
QA_THEMES="$themes" "$native" >"$out/native.txt"
wasmtime run --env "QA_THEMES=$themes" --dir "$themes" "$wasm" >"$out/wasm.txt"
diff -u "$out/native.txt" "$out/wasm.txt"
python3 - "$out/native.txt" <<'PY'
import pathlib, sys, yaml

text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
parts = text.split("===== ")
seen = 0
for part in parts[1:]:
    header, _, body = part.partition("\n")
    header = header.removesuffix("=====").strip()
    _kind, rest = header.split(" ", 1)
    name, _style, _mode = rest.rsplit(" ", 2)
    doc = yaml.safe_load(body)
    if not isinstance(doc, dict) or list(doc) != [name]:
        keys = list(doc) if isinstance(doc, dict) else type(doc).__name__
        raise SystemExit(f"top-level key mismatch for {header}: {keys}")
    theme = doc[name]
    if not isinstance(theme, dict) or "modes" not in theme:
        raise SystemExit(f"{name} is missing modes")
    seen += 1
if seen == 0:
    raise SystemExit("no QA cases")
print(f"pyyaml={seen}")
PY
echo "qa output: $out"
