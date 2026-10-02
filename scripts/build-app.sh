#!/bin/bash
# Builds AlbertAIUsage.app, signs it ad hoc, and installs it to ~/Applications.
set -euo pipefail

cd "$(dirname "$0")/.."
swift build -c release --product AlbertAIUsage

app="build/AlbertAIUsage.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
bin="$(swift build -c release --show-bin-path)"
cp "$bin/AlbertAIUsage" "$app/Contents/MacOS/AlbertAIUsage"
strip -x "$app/Contents/MacOS/AlbertAIUsage"
cp Resources/Info.plist "$app/Contents/Info.plist"
cp Resources/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
cp -R Resources/providers "$app/Contents/Resources/providers"
codesign --force -s - "$app"

mkdir -p "$HOME/Applications"
rm -rf "$HOME/Applications/AlbertAIUsage.app"
cp -R "$app" "$HOME/Applications/"
echo "Installed $HOME/Applications/AlbertAIUsage.app"
