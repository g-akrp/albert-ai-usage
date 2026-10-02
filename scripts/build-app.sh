#!/bin/bash
# Builds "AI Usage.app", signs it ad hoc, and installs it to ~/Applications.
set -euo pipefail

cd "$(dirname "$0")/.."
swift build -c release --product AIUsage

app="build/AI Usage.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
bin="$(swift build -c release --show-bin-path)"
cp "$bin/AIUsage" "$app/Contents/MacOS/AIUsage"
strip -x "$app/Contents/MacOS/AIUsage"
cp Resources/Info.plist "$app/Contents/Info.plist"
cp Resources/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
cp -R Resources/providers "$app/Contents/Resources/providers"
codesign --force -s - "$app"

mkdir -p "$HOME/Applications"
# Versions before 1.3.0 were named AlbertAIUsage.
rm -rf "$HOME/Applications/AI Usage.app" "$HOME/Applications/AlbertAIUsage.app"
cp -R "$app" "$HOME/Applications/"
echo "Installed $HOME/Applications/AI Usage.app"
