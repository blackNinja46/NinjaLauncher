#!/usr/bin/env bash

set -euo pipefail

project_root="$(cd "$(dirname "$0")/.." && pwd)"
source_icon="$project_root/src-tauri/icons/windows-icon-source.png"
icons_dir="$project_root/src-tauri/icons"
generated_dir="$(mktemp -d "${TMPDIR:-/tmp}/ninja-launcher-windows-icons.XXXXXX")"

cleanup() {
  rm -rf "$generated_dir"
}
trap cleanup EXIT

cd "$project_root"
bunx tauri icon --output "$generated_dir" "$source_icon"

for icon in \
  icon.ico \
  Square30x30Logo.png \
  Square44x44Logo.png \
  Square71x71Logo.png \
  Square89x89Logo.png \
  Square107x107Logo.png \
  Square142x142Logo.png \
  Square150x150Logo.png \
  Square284x284Logo.png \
  Square310x310Logo.png \
  StoreLogo.png
do
  install -m 0644 "$generated_dir/$icon" "$icons_dir/$icon"
done
