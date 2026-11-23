#!/bin/sh
# Compile the Icon Composer project into src-tauri/icons/Assets.car (Liquid Glass app icon).
# Needs Xcode 26+; works even when xcode-select points at the Command Line Tools.
set -eu
cd "$(dirname "$0")/.."
: "${DEVELOPER_DIR:=/Applications/Xcode.app/Contents/Developer}"
export DEVELOPER_DIR
out=$(mktemp -d)
xcrun actool icon/PomoSupervisor.icon --compile "$out" \
  --output-format human-readable-text --notices --warnings --errors \
  --output-partial-info-plist "$out/partial.plist" \
  --app-icon PomoSupervisor --include-all-app-icons \
  --enable-on-demand-resources NO --development-region en \
  --target-device mac --minimum-deployment-target 12.0 --platform macosx
cp "$out/Assets.car" src-tauri/icons/Assets.car
rm -rf "$out"
echo "updated src-tauri/icons/Assets.car"
