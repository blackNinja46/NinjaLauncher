#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "$0")/.." && pwd)
icon_bundle="$project_root/src-tauri/icons/macos/NinjaLauncher.icon"
output_directory="$project_root/src-tauri/icons/macos/compiled"
partial_plist="$output_directory/partial-info.plist"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The adaptive macOS icon can only be compiled on macOS with Xcode 26 or newer." >&2
  exit 1
fi

if ! xcrun --find actool >/dev/null 2>&1; then
  echo "Xcode's asset catalog compiler (actool) is required." >&2
  exit 1
fi

mkdir -p "$output_directory"

xcrun actool "$icon_bundle" \
  --compile "$output_directory" \
  --app-icon NinjaLauncher \
  --include-all-app-icons \
  --platform macosx \
  --target-device mac \
  --minimum-deployment-target 26.0 \
  --standalone-icon-behavior all \
  --output-partial-info-plist "$partial_plist" \
  --errors --warnings --notices \
  --output-format human-readable-text

test -f "$output_directory/Assets.car"
echo "Compiled adaptive macOS icon: $output_directory/Assets.car"
