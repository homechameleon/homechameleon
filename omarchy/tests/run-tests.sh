#!/bin/bash
#
# Test harness for HomeChameleon for Omarchy. Runs entirely in a throwaway
# HOME with a local mock of the HomeChameleon HTTP API; touches nothing else.
#
#   tests/run-tests.sh            run everything
#   OMARCHY_CLONE=/path/to/omarchy tests/run-tests.sh
#                                 also use Omarchy's own template renderer and hook runner
#
# Checks pass their arguments to `bash -c '...'` as positional parameters on
# purpose, so the single-quoted snippets must not expand early.
# shellcheck disable=SC2016

set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
SYNC="$ROOT/bin/homechameleon-sync"
SETUP="$ROOT/bin/homechameleon-setup"
OMARCHY_CLONE="${OMARCHY_CLONE:-/workspace/.cache/omarchy}"
TOKEN="test-token-123"

PASS=0
FAIL=0
FAILED_NAMES=()

ok() { PASS=$((PASS + 1)); printf '  ok    %s\n' "$1"; }
nok() { FAIL=$((FAIL + 1)); FAILED_NAMES+=("$1"); printf '  FAIL  %s\n' "$1"; [[ -n ${2:-} ]] && printf '        %s\n' "$2"; }
check() { # check "name" command...
  local name="$1"; shift
  if timeout 60 "$@" >/dev/null 2>&1; then ok "$name"; else nok "$name"; fi
}
section() { printf '\n%s\n' "$1"; }
run() { timeout 60 "$@"; } # never let one command hang the suite

for tool in jq curl python3 base64 timeout; do
  command -v "$tool" >/dev/null || { echo "missing $tool"; exit 2; }
done

TMP=$(mktemp -d)
MOCK_PID=""
cleanup() {
  [[ -n $MOCK_PID ]] && kill "$MOCK_PID" 2>/dev/null
  rm -rf "$TMP"
}
trap cleanup EXIT

export HOME="$TMP/home"
export HOMECHAMELEON_NO_SYSTEMCTL=1
export HOMECHAMELEON_NO_OMARCHY_CLI=1
unset HOMECHAMELEON_CONFIG HOMECHAMELEON_TOKEN HOMECHAMELEON_STRICT
PATH="$ROOT/bin:$PATH"

CURRENT="$HOME/.local/state/omarchy/current"
CONFIG="$HOME/.config/homechameleon/config"
LOG="$HOME/.local/state/homechameleon/sync.log"
RECORD="$TMP/requests.jsonl"
HOOK="$HOME/.config/omarchy/hooks/theme-set.d/homechameleon-sync"

# --------------------------------------------------------------- fixtures

DARK_COLORS="$ROOT/tests/fixtures/tokyo-night.colors.toml"
LIGHT_COLORS="$ROOT/tests/fixtures/catppuccin-latte.colors.toml"
if [[ -f $OMARCHY_CLONE/themes/tokyo-night/colors.toml ]]; then
  DARK_COLORS="$OMARCHY_CLONE/themes/tokyo-night/colors.toml"
  LIGHT_COLORS="$OMARCHY_CLONE/themes/catppuccin-latte/colors.toml"
fi

use_theme() { # use_theme <name> <colors.toml>
  rm -rf "$CURRENT/theme"
  mkdir -p "$CURRENT/theme/backgrounds"
  cp "$2" "$CURRENT/theme/colors.toml"
  printf '%s\n' "$1" >"$CURRENT/theme.name"
}

im() { if command -v magick >/dev/null; then magick "$@"; else convert "$@"; fi; }
dims() { if command -v magick >/dev/null; then magick identify -format '%w %h' "$1"; else identify -format '%w %h' "$1"; fi; }

mkdir -p "$HOME" "$CURRENT"
use_theme tokyo-night "$DARK_COLORS"

HAVE_MAGICK=0
if command -v magick >/dev/null || command -v convert >/dev/null; then HAVE_MAGICK=1; fi
HAVE_FFMPEG=0
command -v ffmpeg >/dev/null && HAVE_FFMPEG=1

WALL_IMG="$TMP/wall.png"
WALL_VID="$TMP/wall.mp4"
((HAVE_MAGICK)) && im -size 3000x1500 gradient:'#1a1b26-#7aa2f7' "$WALL_IMG"
((HAVE_FFMPEG)) && run ffmpeg -nostdin -v error -y -f lavfi -i testsrc=size=1280x720:rate=10 -t 2 -pix_fmt yuv420p "$WALL_VID"
[[ -f $WALL_IMG ]] && ln -nsf "$WALL_IMG" "$CURRENT/background"

# --------------------------------------------------------------- mock server

python3 "$ROOT/tests/mock_server.py" 0 "$RECORD" "$TMP/port" &
MOCK_PID=$!
for _ in $(seq 50); do [[ -s $TMP/port ]] && break; sleep 0.1; done
PORT=$(cat "$TMP/port" 2>/dev/null)
[[ -n $PORT ]] || { echo "mock server did not start"; exit 2; }
URL="http://127.0.0.1:$PORT"

last_request() { tail -n1 "$RECORD"; }
request_count() { if [[ -f $RECORD ]]; then wc -l <"$RECORD"; else echo 0; fi; }
set_config() { sed -i "s|^$1=.*|$1=$2|" "$CONFIG"; } # set_config KEY VALUE

# =============================================================== tests

section "Static checks"
if command -v shellcheck >/dev/null; then
  if out=$(shellcheck "$ROOT"/bin/* "$ROOT"/tests/*.sh 2>&1); then ok "shellcheck clean (bin/, tests/)"; else nok "shellcheck clean" "$out"; fi
else
  nok "shellcheck clean" "shellcheck is not installed"
fi
check "bash -n homechameleon-sync" bash -n "$SYNC"
check "bash -n homechameleon-setup" bash -n "$SETUP"
check "--help works" "$SYNC" --help
if hits=$(grep -nE '\bHA\b' "$ROOT"/bin/* "$ROOT"/README.md "$ROOT"/templates/* "$ROOT"/PKGBUILD 2>/dev/null); then
  nok "no 'HA' abbreviation in user-facing files" "$hits"
else ok "no 'HA' abbreviation in user-facing files"; fi
if hits=$(grep -niE 'studio|profile|gallery' "$ROOT"/templates/* 2>/dev/null); then
  nok "template avoids studio/profile/gallery" "$hits"
else ok "template avoids studio/profile/gallery"; fi

section "Unconfigured hook run"
run "$SYNC" tokyo-night >/dev/null 2>&1; rc=$?
check "exits 0 without a config (silent during theme switches)" test "$rc" -eq 0
check "logs that it is not configured" grep -q "no config" "$LOG"
check "sends nothing" test "$(request_count)" -eq 0

section "Setup (non-interactive, background off)"
printf '%s\n' "$TOKEN" >"$TMP/token"
out=$(run "$SETUP" --url "$URL/" --token-file "$TMP/token" --style sheer --background off --template --no-path-unit --no-test 2>&1); rc=$?
check "setup exits 0" test "$rc" -eq 0
check "config written" test -f "$CONFIG"
check "config is mode 600" test "$(stat -c %a "$CONFIG")" = 600
check "config dir is mode 700" test "$(stat -c %a "$(dirname "$CONFIG")")" = 700
check "BACKGROUND=0 by default" grep -qx 'BACKGROUND=0' "$CONFIG"
check "STYLE=sheer" grep -qx 'STYLE=sheer' "$CONFIG"
check "trailing slash stripped from URL" grep -qx "URL=$URL" "$CONFIG"
check "hook installed in theme-set.d" test -x "$HOOK"
check "hook is a two-line wrapper" test "$(wc -l <"$HOOK")" -eq 2
check "hook execs homechameleon-sync" grep -qE '^exec .*homechameleon-sync --if-changed "\$@"$' "$HOOK"
check "template installed" test -f "$HOME/.config/omarchy/themed/homechameleon.json.tpl"
check "no privacy note when background is off" bash -c '! grep -q "Privacy note" <<<"$1"' _ "$out"

section "Dry run (background off)"
DRY="$TMP/dry.json"
run "$SYNC" --dry-run >"$DRY" 2>"$TMP/dry.err"; rc=$?
check "dry run exits 0" test "$rc" -eq 0
check "dry run sends nothing" test "$(request_count)" -eq 0
check "payload is valid JSON" jq -e . "$DRY"
check "top-level keys match frozen v1 (no images)" jq -e '(keys - ["apply","background","mode","name","options","palette","set_default","style"]) == [] and (keys | length) == 8' "$DRY"
check "name=Omarchy, style=sheer, mode=dark" jq -e '.name == "Omarchy" and .style == "sheer" and .mode == "dark"' "$DRY"
check "palette.format=omarchy-colors-toml" jq -e '.palette.format == "omarchy-colors-toml"' "$DRY"
check "palette.data is the raw colors.toml text" bash -c 'diff <(jq -j .palette.data "$1") "$2"' _ "$DRY" "$CURRENT/theme/colors.toml"
check "background block is {enabled:false, blur:0, dim:0.0, max_edge:2560}" jq -e '.background == {"enabled":false,"blur":0,"dim":0.0,"max_edge":2560}' "$DRY"
check "options block" jq -e '.options == {"bubble":true,"mushroom":true,"card_mod":false,"contrast":"AA"}' "$DRY"
check "apply and set_default true" jq -e '.apply == true and .set_default == true' "$DRY"
check "token is not in the payload" bash -c '! grep -q "$1" "$2"' _ "$TOKEN" "$DRY"

section "Generate (background off)"
run "$SYNC" tokyo-night >"$TMP/gen.out" 2>&1; rc=$?
check "generate exits 0" test "$rc" -eq 0
check "one request sent" test "$(request_count)" -eq 1
REQ=$(last_request)
check "POST /api/homechameleon/generate" jq -e '.method == "POST" and .path == "/api/homechameleon/generate"' <<<"$REQ"
check "Authorization: Bearer <token>" jq -e --arg t "Bearer $TOKEN" '.authorization == $t' <<<"$REQ"
check "Content-Type: application/json" jq -e '.content_type == "application/json"' <<<"$REQ"
check "sent body equals the dry-run body" bash -c 'diff <(jq -S .body <<<"$1") <(jq -S . "$2")' _ "$REQ" "$DRY"
check "no images key when BACKGROUND=0" jq -e '.body | has("images") | not' <<<"$REQ"
check "response logged" grep -q 'generate response: {"theme":"Omarchy"' "$LOG"
check "summary printed" grep -q "Theme 'Omarchy' (style sheer): applied=true, default=true, contrast pass" "$TMP/gen.out"
check "token never written to the log" bash -c '! grep -q "$1" "$2"' _ "$TOKEN" "$LOG"

section "--if-changed"
run "$SYNC" --if-changed tokyo-night >/dev/null 2>&1
check "identical second run is skipped" test "$(request_count)" -eq 1
check "skip is logged" grep -q "unchanged since the last sync" "$LOG"
run "$SYNC" tokyo-night >/dev/null 2>&1
check "without --if-changed it always sends" test "$(request_count)" -eq 2

section "Light theme"
use_theme catppuccin-latte "$LIGHT_COLORS"
check "mode=light comes from colors.toml" bash -c '"$1" --dry-run | jq -e ".mode == \"light\""' _ "$SYNC"
printf 'accent = "#123456"\nbackground = "#ffffff"\n' >"$CURRENT/theme/colors.toml"
touch "$CURRENT/theme/light.mode"
check "light.mode marker used when the mode key is missing" bash -c '"$1" --dry-run | jq -e ".mode == \"light\""' _ "$SYNC"
rm "$CURRENT/theme/light.mode"
check "defaults to dark without mode key or marker" bash -c '"$1" --dry-run | jq -e ".mode == \"dark\""' _ "$SYNC"
use_theme tokyo-night "$DARK_COLORS"

section "Warnings and style list"
set_config STYLE starship
run "$SYNC" >"$TMP/warn.out" 2>&1
check "server warnings are shown" grep -q "Home Assistant: Starship Console needs card-mod" "$TMP/warn.out"
check "server warnings are logged" grep -q "\[warn\] Home Assistant: Starship Console needs card-mod" "$LOG"
check "style is a free value (starship sent)" jq -e '.body.style == "starship"' <<<"$(last_request)"
set_config STYLE my-future_style
check "unknown style ids pass through unchanged" bash -c '"$1" --dry-run | jq -e ".style == \"my-future_style\""' _ "$SYNC"
set_config STYLE sheer
run "$SYNC" --styles >"$TMP/styles.out" 2>&1; rc=$?
check "--styles exits 0" test "$rc" -eq 0
check "--styles calls GET /api/homechameleon/styles" jq -e '.method == "GET" and .path == "/api/homechameleon/styles"' <<<"$(last_request)"
check "--styles lists the launch ids" bash -c 'for s in sheer bubbles material starship; do grep -q "^$s" "$1" || exit 1; done' _ "$TMP/styles.out"

section "Preview"
run "$SYNC" --preview >"$TMP/preview.out" 2>&1; rc=$?
check "--preview exits 0" test "$rc" -eq 0
check "--preview calls POST /api/homechameleon/preview" jq -e '.method == "POST" and .path == "/api/homechameleon/preview"' <<<"$(last_request)"
check "--preview sends the same v1 body" jq -e '.body.palette.format == "omarchy-colors-toml" and .body.name == "Omarchy"' <<<"$(last_request)"
check "--preview reports that nothing was written" grep -q "Nothing was written" "$TMP/preview.out"

section "Background on"
if ((HAVE_MAGICK)) && [[ -f $WALL_IMG ]]; then
  out=$(run "$SETUP" --url "$URL" --token-file "$TMP/token" --style sheer --background on --no-template --path-unit --no-test 2>&1)
  check "setup prints the privacy note" grep -q "Home Assistant serves" <<<"$out"
  check "privacy note mentions /local without a login" grep -q "/local files WITHOUT a login" <<<"$out"
  check "BACKGROUND=1 written" grep -qx 'BACKGROUND=1' "$CONFIG"
  check "previous config backed up" bash -c 'ls "$1"/*/.config/homechameleon/config' _ "$HOME/.local/state/homechameleon/backups"
  check "path unit installed" test -f "$HOME/.config/systemd/user/homechameleon-sync.path"
  check "service unit points at the sync binary" grep -q "ExecStart=$(readlink -f "$SYNC") --if-changed --quiet" "$HOME/.config/systemd/user/homechameleon-sync.service"

  run "$SYNC" >/dev/null 2>&1
  REQ=$(last_request)
  check "images[] sent when BACKGROUND=1" jq -e '.body.images | length == 1' <<<"$REQ"
  check "images[0] is {data, weight 1.0, use_as_background true}" jq -e '.body.images[0] | .weight == 1.0 and .use_as_background == true and (keys == ["data","use_as_background","weight"])' <<<"$REQ"
  check "background.enabled=true" jq -e '.body.background.enabled == true' <<<"$REQ"
  jq -r '.body.images[0].data' <<<"$REQ" | base64 -d >"$TMP/sent.webp"
  check "image is a WebP" bash -c '[[ $(head -c4 "$1") == RIFF && $(dd if="$1" bs=1 skip=8 count=4 2>/dev/null) == WEBP ]]' _ "$TMP/sent.webp"
  check "downscaled 3000x1500 to 2560x1280 (longest edge 2560)" test "$(dims "$TMP/sent.webp")" = "2560 1280"
  check "response lists the background file" grep -q 'www/homechameleon/omarchy/bg-' "$LOG"

  set_config MAX_EDGE 800
  run "$SYNC" >/dev/null 2>&1
  jq -r '.body.images[0].data' <<<"$(last_request)" | base64 -d >"$TMP/small.webp"
  check "MAX_EDGE=800 honoured (800x400, max_edge 800)" bash -c '[[ $1 == "800 400" ]] && jq -e ".body.background.max_edge == 800" <<<"$2"' _ "$(dims "$TMP/small.webp")" "$(last_request)"
  set_config MAX_EDGE 2560

  check "dry run replaces image data with a placeholder" bash -c '"$1" --dry-run | jq -e ".images[0].data | startswith(\"<base64 WebP\")"' _ "$SYNC"

  if ((HAVE_FFMPEG)); then
    ln -nsf "$WALL_VID" "$CURRENT/background"
    run "$SYNC" >/dev/null 2>&1
    jq -r '.body.images[0].data' <<<"$(last_request)" | base64 -d >"$TMP/video.webp"
    check "video background -> ffmpeg frame -> WebP (1280x720)" test "$(dims "$TMP/video.webp")" = "1280 720"
  else
    nok "video background" "ffmpeg not installed"
  fi

  ln -nsf "$TMP/missing.png" "$CURRENT/background"
  run "$SYNC" >/dev/null 2>&1; rc=$?
  check "missing background: still exits 0" test "$rc" -eq 0
  check "missing background: palette sent without images, enabled=false" jq -e '(.body | has("images") | not) and .body.background.enabled == false' <<<"$(last_request)"
  ln -nsf "$WALL_IMG" "$CURRENT/background"
  set_config BACKGROUND 0
else
  nok "background tests" "ImageMagick not installed"
fi

section "Export"
EXPORT="$HOME/Downloads/homechameleon-palette-tokyo-night.json"
before=$(request_count)
run "$SYNC" --export >/dev/null 2>&1; rc=$?
check "--export exits 0" test "$rc" -eq 0
check "--export writes to ~/Downloads" test -f "$EXPORT"
check "schema homechameleon-palette/1, source omarchy, theme, mode" jq -e '.schema == "homechameleon-palette/1" and (has("version") | not) and .source == "omarchy" and .theme == "tokyo-night" and .mode == "dark"' "$EXPORT"
check "fallback export always has color0-color15" jq -e '[range(16) | "color\(.)"] - (.colors | keys) == []' "$EXPORT"
check "fallback ANSI order (color0=background, color1=red, color7=foreground, color8=muted, color15=bright_foreground)" jq -e '.colors | .color0 == .background and .color1 == .red and .color7 == .foreground and .color8 == .muted and .color9 == .bright_red and .color15 == .bright_foreground' "$EXPORT"
check "export logged its palette source (colors.toml)" grep -q "export: palette from colors.toml" "$LOG"
cp "$EXPORT" "$TMP/export-fallback.json"
check "colors use Omarchy key names" jq -e '.colors.accent == "#7aa2f7" and .colors.background == "#1a1b26" and .colors.bright_magenta == "#bb9af7"' "$EXPORT"
check "no studio/profile/gallery words in export" bash -c '! grep -qiE "studio|profile|gallery" "$1"' _ "$EXPORT"
check "--export sends nothing" test "$(request_count)" -eq "$before"
check "no wallpaper exported when BACKGROUND=0" bash -c '! ls "$1"/*.webp' _ "$HOME/Downloads"
if ((HAVE_MAGICK)); then
  run "$SYNC" --export "$TMP/out" --with-wallpaper >/dev/null 2>&1
  check "--export DIR --with-wallpaper writes JSON and WebP" bash -c 'test -f "$1/homechameleon-palette-tokyo-night.json" && test -f "$1/homechameleon-tokyo-night-background.webp" && jq -e ".background_file == \"homechameleon-tokyo-night-background.webp\" and (has(\"background\") | not)" "$1/homechameleon-palette-tokyo-night.json"' _ "$TMP/out"
fi

section "Template rendered by Omarchy's own renderer"
if [[ -f $OMARCHY_CLONE/bin/omarchy-theme-set-templates ]] && command -v gawk >/dev/null; then
  NEXT="$CURRENT/next-theme"
  rm -rf "$NEXT"; mkdir -p "$NEXT"
  cp "$CURRENT/theme/colors.toml" "$NEXT/"
  OMARCHY_PATH="$OMARCHY_CLONE" PATH="$OMARCHY_CLONE/bin:$PATH" run bash "$OMARCHY_CLONE/bin/omarchy-theme-set-templates" >/dev/null 2>&1
  check "renderer produced homechameleon.json" test -f "$NEXT/homechameleon.json"
  check "rendered JSON is valid" jq -e . "$NEXT/homechameleon.json"
  check "no unrendered {{ }} left" bash -c '! grep -q "{{" "$1"' _ "$NEXT/homechameleon.json"
  check "rendered values match colors.toml" jq -e '.mode == "dark" and .colors.accent == "#7aa2f7" and .colors.color0 != ""' "$NEXT/homechameleon.json"
  cp "$NEXT/homechameleon.json" "$CURRENT/theme/homechameleon.json"
  rm -rf "$NEXT"
  run "$SYNC" --export "$TMP/out2" >/dev/null 2>&1
  check "--export prefers the rendered JSON (has color0..15)" jq -e '(.colors | has("color15")) and .theme == "tokyo-night" and .schema == "homechameleon-palette/1"' "$TMP/out2/homechameleon-palette-tokyo-night.json"
  check "export logged the rendered source" grep -q "export: palette from rendered" "$LOG"
  check "rendered export == fallback export (byte for byte)" cmp "$TMP/out2/homechameleon-palette-tokyo-night.json" "$TMP/export-fallback.json"
  rm -f "$CURRENT/theme/homechameleon.json"

  mkdir -p "$TMP/otc-bin"
  install -m 755 "$OMARCHY_CLONE/bin/omarchy-theme-color" "$TMP/otc-bin/"
  PATH="$TMP/otc-bin:$PATH" run "$SYNC" --export "$TMP/out3" >/dev/null 2>&1
  check "export logged the omarchy-theme-color source" grep -q "export: palette from omarchy-theme-color" "$LOG"
  check "omarchy-theme-color export == fallback export (byte for byte)" cmp "$TMP/out3/homechameleon-palette-tokyo-night.json" "$TMP/export-fallback.json"
  use_theme catppuccin-latte "$LIGHT_COLORS"
  run "$SYNC" --export "$TMP/out4" >/dev/null 2>&1
  PATH="$TMP/otc-bin:$PATH" run "$SYNC" --export "$TMP/out5" >/dev/null 2>&1
  check "light theme: fallback == omarchy-theme-color export" cmp "$TMP/out4/homechameleon-palette-catppuccin-latte.json" "$TMP/out5/homechameleon-palette-catppuccin-latte.json"
  check "light theme export has mode light and color0-15" jq -e '.mode == "light" and ([range(16) | "color\(.)"] - (.colors | keys) == [])' "$TMP/out4/homechameleon-palette-catppuccin-latte.json"
  use_theme tokyo-night "$DARK_COLORS"
else
  printf '  skip  Omarchy clone not found at %s (set OMARCHY_CLONE)\n' "$OMARCHY_CLONE"
fi

section "Theme name"
check "default name is Omarchy" bash -c '"$1" --dry-run | jq -e ".name == \"Omarchy\""' _ "$SYNC"
set_config NAME "Living Room"
check "NAME= override is sent as is" bash -c '"$1" --dry-run | jq -e ".name == \"Living Room\""' _ "$SYNC"
set_config NAME "@theme"
check "NAME=@theme sends the Omarchy theme name (Tokyo Night)" bash -c '"$1" --dry-run | jq -e ".name == \"Tokyo Night\""' _ "$SYNC"
check "NAME=@theme prefers the slug passed by the hook" bash -c '"$1" --dry-run catppuccin-latte | jq -e ".name == \"Catppuccin Latte\""' _ "$SYNC"
set_config NAME Omarchy
cp "$CONFIG" "$TMP/config.before-name"
run "$SETUP" --url "$URL" --token-file "$TMP/token" --name @theme --background off --no-template --no-path-unit --no-hook --no-test >/dev/null 2>&1
check "setup --name @theme writes NAME=@theme" grep -qx 'NAME=@theme' "$CONFIG"
run "$SETUP" --url "$URL" --token-file "$TMP/token" --background off --no-template --no-path-unit --no-hook --no-test >/dev/null 2>&1
check "setup keeps the existing name when none is given" grep -qx 'NAME=@theme' "$CONFIG"
cp "$TMP/config.before-name" "$CONFIG"; chmod 600 "$CONFIG"

section "Failure handling"
cp "$CONFIG" "$TMP/config.413"
set_config NAME TooBig
run "$SYNC" >/dev/null 2>"$TMP/413.err"; rc=$?
check "413: hook mode still exits 0" test "$rc" -eq 0
check "413: plain-words message about the 10 MB image limit" grep -q "background image is too big (over 10 MB, HTTP 413)" "$TMP/413.err"
check "413: message is logged" grep -q "over 10 MB, HTTP 413" "$LOG"
cp "$TMP/config.413" "$CONFIG"; chmod 600 "$CONFIG"
cp "$CONFIG" "$TMP/config.good"
set_config TOKEN wrong-token
run "$SYNC" >/dev/null 2>"$TMP/bad.err"; rc=$?
check "bad token: hook mode still exits 0" test "$rc" -eq 0
check "bad token: 401 explained" grep -q "rejected the token (401)" "$TMP/bad.err"
run "$SYNC" --strict >/dev/null 2>&1; rc=$?
check "bad token with --strict exits non-zero" test "$rc" -ne 0
cp "$TMP/config.good" "$CONFIG"; chmod 600 "$CONFIG"
set_config URL "http://127.0.0.1:9"
run "$SYNC" >/dev/null 2>"$TMP/down.err"; rc=$?
check "server down: exits 0" test "$rc" -eq 0
check "server down: explained" grep -q "could not reach" "$TMP/down.err"
cp "$TMP/config.good" "$CONFIG"
chmod 644 "$CONFIG"
run "$SYNC" --dry-run >/dev/null 2>"$TMP/perm.err"
check "warns when the config is not chmod 600" grep -q "chmod 600" "$TMP/perm.err"
chmod 600 "$CONFIG"
printf 'URL=%s\nTOKEN=%s\n$(touch %s/pwned)\nSTYLE=$(touch %s/pwned2)\n' "$URL" "$TOKEN" "$TMP" "$TMP" >"$TMP/evil.config"; chmod 600 "$TMP/evil.config"
HOMECHAMELEON_CONFIG="$TMP/evil.config" run "$SYNC" --dry-run >/dev/null 2>&1
check "config is parsed, never executed" bash -c 'test ! -e "$1/pwned" && test ! -e "$1/pwned2"' _ "$TMP"
mv "$CURRENT/theme/colors.toml" "$TMP/colors.bak"
run "$SYNC" >/dev/null 2>&1; rc=$?
check "missing colors.toml: exits 0 in hook mode" test "$rc" -eq 0
check "missing colors.toml: explained in the log" grep -q "Omarchy v4 is required" "$LOG"
mv "$TMP/colors.bak" "$CURRENT/theme/colors.toml"

section "Real Omarchy hook runner and installer"
if [[ -f $OMARCHY_CLONE/bin/omarchy-hook && -f $OMARCHY_CLONE/bin/omarchy-hook-install ]]; then
  # Expose only the two Omarchy commands this needs, not the whole clone.
  mkdir -p "$TMP/omarchy-bin"
  install -m 755 "$OMARCHY_CLONE/bin/omarchy-hook" "$OMARCHY_CLONE/bin/omarchy-hook-install" "$TMP/omarchy-bin/"
  rm -f "$HOOK"
  HOMECHAMELEON_NO_OMARCHY_CLI=0 PATH="$TMP/omarchy-bin:$PATH" run "$SETUP" --url "$URL" --token-file "$TMP/token" \
    --style bubbles --background off --no-template --no-path-unit --no-test >/dev/null 2>&1
  check "omarchy-hook-install placed the wrapper" test -x "$HOOK"
  before=$(request_count)
  out=$(PATH="$TMP/omarchy-bin:$PATH" run bash "$TMP/omarchy-bin/omarchy-hook" theme-set tokyo-night 2>&1)
  check "omarchy-hook theme-set ran the wrapper without 'Hook failed'" bash -c '! grep -q "Hook failed" <<<"$1"' _ "$out"
  check "theme switch produced one generate request" test "$(request_count)" -eq $((before + 1))
  check "request carries the configured style" jq -e '.body.style == "bubbles" and .path == "/api/homechameleon/generate"' <<<"$(last_request)"
else
  printf '  skip  Omarchy clone not found at %s (set OMARCHY_CLONE)\n' "$OMARCHY_CLONE"
fi

section "Uninstall"
run "$SETUP" --uninstall --yes >"$TMP/uninstall.out" 2>&1; rc=$?
check "uninstall exits 0" test "$rc" -eq 0
check "config removed" test ! -e "$CONFIG"
check "hook removed" test ! -e "$HOOK"
check "template removed" test ! -e "$HOME/.config/omarchy/themed/homechameleon.json.tpl"
check "units removed" test ! -e "$HOME/.config/systemd/user/homechameleon-sync.path"
LATEST=$(find "$HOME/.local/state/homechameleon/backups" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -n | tail -n1 | cut -d' ' -f2-)
check "config backed up before removal" test -f "$LATEST/.config/homechameleon/config"
check "hook backed up before removal" test -f "$LATEST/.config/omarchy/hooks/theme-set.d/homechameleon-sync"
check "units backed up before removal" test -f "$LATEST/.config/systemd/user/homechameleon-sync.service"
check "backups are private (700)" test "$(stat -c %a "$LATEST")" = 700
check "log kept" test -f "$LOG"

# --------------------------------------------------------------- summary
printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
if ((FAIL)); then
  printf 'Failed: %s\n' "${FAILED_NAMES[@]}"
  exit 1
fi
