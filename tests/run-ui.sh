#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fixture_dir=$(mktemp -d /tmp/opennav-ui.XXXXXX)
trap 'rm -rf "$fixture_dir"' EXIT
export XDG_DATA_HOME="$fixture_dir/data"
export XDG_CONFIG_HOME="$fixture_dir/config"
export XDG_DATA_DIRS="$fixture_dir/data"
export GSETTINGS_SCHEMA_DIR=/usr/share/glib-2.0/schemas
export OPENNAV_TEST_CAPTURE="$fixture_dir/launch.txt"
mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME"
cat > "$fixture_dir/capture" <<'CAPTURE'
#!/bin/sh
printf '%s\n' "$@" > "$OPENNAV_TEST_CAPTURE"
CAPTURE
chmod +x "$fixture_dir/capture"
for browser in firefox microsoft-edge google-chrome; do
    cat > "$XDG_DATA_HOME/applications/$browser-test.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=$browser Test
Exec=$fixture_dir/capture $browser %u
Icon=web-browser
MimeType=x-scheme-handler/http;x-scheme-handler/https;
DESKTOP
done
cat > "$XDG_CONFIG_HOME/mimeapps.list" <<'MIME'
[Default Applications]
x-scheme-handler/http=firefox-test.desktop
x-scheme-handler/https=firefox-test.desktop
[Added Associations]
x-scheme-handler/http=firefox-test.desktop;microsoft-edge-test.desktop;google-chrome-test.desktop;
x-scheme-handler/https=firefox-test.desktop;microsoft-edge-test.desktop;google-chrome-test.desktop;
MIME
update-desktop-database "$XDG_DATA_HOME/applications"
cargo test --release keyboard_navigation_preserves_selection -- --ignored --test-threads=1
