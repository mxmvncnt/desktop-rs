#!/usr/bin/env bash
# Usage: build-aux/flatpak/run-dev.sh [app args]
# Builds the working tree as a separate "desktop-rs (Dev)" Flatpak (own app id,
# so own sandbox data and no clash with an installed release), installs it for
# the current user and runs it. Needs flatpak, flatpak-builder and the flathub
# remote; missing runtimes are installed.
set -euo pipefail
cd "$(dirname "$0")/../.."

id=io.github.mxmvncnt.desktop-rs
dev=$id-dev
out=$PWD/target/flatpak
mkdir -p "$out/cargo" "$out/target"

flatpak install --user --noninteractive --or-update flathub \
  org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08

# Derive the dev manifest: new app id, renamed/edited desktop entry, metainfo and icon,
# and Cargo home/target kept on the host so crates and incremental builds are cached.
sed \
  -e "s|^app-id: .*|app-id: $dev|" \
  -e "s|path: ../..|path: $PWD|" \
  -e "s|CARGO_HOME: .*|CARGO_HOME: $out/cargo|" \
  -e "s|^        - --share=network|&\n        - --filesystem=$out\n      env:\n        CARGO_TARGET_DIR: $out/target|" \
  -e "s|target/release/desktop-rs|$out/target/release/desktop-rs|" \
  -e "s|^\(.*\)install -Dm644 build-aux/flatpak/$id.desktop -t /app/share/applications|\1install -Dm644 build-aux/flatpak/$id.desktop /app/share/applications/$dev.desktop \&\& sed -i -e 's/^Name=.*/Name=desktop-rs (Dev)/' -e 's/^Icon=.*/Icon=$dev/' /app/share/applications/$dev.desktop|" \
  -e "s#^\(.*\)install -Dm644 build-aux/flatpak/$id.metainfo.xml -t /app/share/metainfo#\1install -Dm644 build-aux/flatpak/$id.metainfo.xml /app/share/metainfo/$dev.metainfo.xml \&\& sed -i -e 's/$id/$dev/g' -e 's|<name>.*</name>|<name>desktop-rs (Dev)</name>|' /app/share/metainfo/$dev.metainfo.xml#" \
  -e "s|apps/$id.png|apps/$dev.png|" \
  build-aux/flatpak/$id.yaml > "$out/$dev.yaml"

# --state-dir keeps the git build and downloads cached between runs.
flatpak-builder --user --install --force-clean \
  --state-dir="$out/state" --install-deps-from=flathub \
  "$out/build" "$out/$dev.yaml"

flatpak run "$dev" "$@"
