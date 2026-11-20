#!/bin/bash
# Builds "AI Usage.app" (or "AI Usage Beta.app" with --beta [version]), signs it ad hoc, and installs to ~/Applications.
set -euo pipefail

beta=false
beta_version=""
if [[ "${1:-}" == "--beta" ]]; then
    beta=true
    beta_version="${2:-}"
elif [[ -n "${1:-}" ]]; then
    echo "usage: scripts/build-app.sh [--beta [version]]" >&2
    exit 2
fi

cd "$(dirname "$0")/.."
swift build -c release --product AIUsage

bin="$(swift build -c release --show-bin-path)"
if $beta; then
    app_name="AI Usage Beta.app"
    exec_name="AIUsageBeta"
else
    app_name="AI Usage.app"
    exec_name="AIUsage"
fi

app="build/$app_name"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin/AIUsage" "$app/Contents/MacOS/$exec_name"
strip -x "$app/Contents/MacOS/$exec_name"
cp Resources/Info.plist "$app/Contents/Info.plist"
cp Resources/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
cp -R Resources/providers "$app/Contents/Resources/providers"

if $beta; then
    plist="$app/Contents/Info.plist"
    current=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$plist")
    build=$(/usr/libexec/PlistBuddy -c "Print :CFBundleVersion" "$plist")
    if [[ -z "$beta_version" ]]; then
        IFS='.' read -r major minor patch <<< "$current"
        beta_version="${major}.$((minor + 1)).0-beta.1"
    fi
    /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier local.ai-usage.beta" "$plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleName AI Usage Beta" "$plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName AI Usage Beta" "$plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $exec_name" "$plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $beta_version" "$plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $((build + 1))" "$plist"
fi

codesign --force -s - "$app"

mkdir -p "$HOME/Applications"
if $beta; then
    rm -rf "$HOME/Applications/$app_name"
    cp -R "$app" "$HOME/Applications/"
    echo "Installed $HOME/Applications/$app_name ($beta_version, bundle id local.ai-usage.beta)"
else
    # Versions before 1.3.0 were named AlbertAIUsage.
    rm -rf "$HOME/Applications/AI Usage.app" "$HOME/Applications/AlbertAIUsage.app"
    cp -R "$app" "$HOME/Applications/"
    echo "Installed $HOME/Applications/AI Usage.app"
fi
