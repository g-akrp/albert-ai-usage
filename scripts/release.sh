#!/bin/bash
# Builds a release DMG: sets the version, runs checks, builds and signs the app, makes the DMG,
# commits, and tags. With --publish, also pushes and creates the GitHub release.
#
# Usage: scripts/release.sh <version> [--publish]     e.g. scripts/release.sh 1.1.0
#
# If HOMEBREW_TAP_DIR points at a checkout of a Homebrew tap repo, Casks/ai-usage.rb there is rewritten
# for this version and committed; with --publish it is pushed after the GitHub release exists.
set -euo pipefail

version="${1:-}"
publish="${2:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "usage: scripts/release.sh <major.minor.patch> [--publish]" >&2
    exit 2
fi
if [[ -n "$publish" && "$publish" != "--publish" ]]; then
    echo "unknown option: $publish" >&2
    exit 2
fi

cd "$(dirname "$0")/.."
if [[ "$(git branch --show-current)" != "main" ]]; then
    echo "release from main only" >&2
    exit 1
fi
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    echo "working tree is not clean" >&2
    exit 1
fi
if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
    echo "tag v$version already exists" >&2
    exit 1
fi

# Version: Info.plist (short version and build number) and the app constant.
plist=Resources/Info.plist
versionfile=Sources/AIUsageCore/Version.swift
# Undo the version edits if anything below fails before the release commit.
trap 'git checkout -- "$plist" "$versionfile"' ERR
current=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$plist")
build=$(/usr/libexec/PlistBuddy -c "Print :CFBundleVersion" "$plist")
# The first release ships the version already in the files; later ones bump the build number.
if [[ "$current" != "$version" ]]; then
    build=$((build + 1))
fi
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build" "$plist"
sed -i '' -E "s/(current = \")[0-9]+\.[0-9]+\.[0-9]+(\")/\1$version\2/" "$versionfile"
grep -q "current = \"$version\"" "$versionfile"

swift run CoreChecks
scripts/build-app.sh

# DMG with the app and an Applications shortcut for drag-to-install.
dmg="build/AIUsage-$version.dmg"
staging=build/dmg
rm -rf "$staging" "$dmg"
mkdir -p "$staging"
ditto "build/AI Usage.app" "$staging/AI Usage.app"
ln -s /Applications "$staging/Applications"
hdiutil create -volname "AI Usage $version" -srcfolder "$staging" -fs HFS+ -format UDZO -ov "$dmg" >/dev/null
rm -rf "$staging"
(cd build && shasum -a 256 "AIUsage-$version.dmg" > "AIUsage-$version.dmg.sha256")

# Rewrites the Homebrew cask in the tap checkout for this version and commits it.
update_cask() {
    local sha cask
    sha=$(cut -d' ' -f1 "build/AIUsage-$version.dmg.sha256")
    mkdir -p "$HOMEBREW_TAP_DIR/Casks"
    cask="$HOMEBREW_TAP_DIR/Casks/ai-usage.rb"
    cat > "$cask" <<CASK
cask "ai-usage" do
  version "$version"
  sha256 "$sha"

  url "https://github.com/g-akrp/albert-ai-usage/releases/download/v#{version}/AIUsage-#{version}.dmg"
  name "AI Usage"
  desc "Menu bar usage meters for AI coding tools"
  homepage "https://github.com/g-akrp/albert-ai-usage"

  depends_on macos: :ventura

  app "AI Usage.app"

  # Ad-hoc signed, not notarized: clear the quarantine flag so Gatekeeper lets it open.
  postflight_steps do
    run "/usr/bin/xattr",
        args:           ["-dr", "com.apple.quarantine", "{{appdir}}/AI Usage.app"],
        writable_paths: ["{{appdir}}/AI Usage.app"]
  end

  zap trash: "~/Library/Preferences/local.ai-usage.plist"
end
CASK
    git -C "$HOMEBREW_TAP_DIR" add Casks/ai-usage.rb
    git -C "$HOMEBREW_TAP_DIR" commit -q -m "ai-usage $version"
}

git add "$plist" "$versionfile"
git commit -q --allow-empty -m "release: $version"
trap - ERR
git tag -a "v$version" -m "AI Usage $version"
echo "Built $dmg"
cat "build/AIUsage-$version.dmg.sha256"

if [[ "$publish" == "--publish" ]]; then
    # Use the gh login for this push, whatever credential helper git is configured with.
    git -c credential.helper= -c 'credential.helper=!gh auth git-credential' push origin main "v$version"
    gh release create "v$version" "$dmg" "build/AIUsage-$version.dmg.sha256" \
        --title "AI Usage $version" --generate-notes
    if [[ -n "${HOMEBREW_TAP_DIR:-}" ]]; then
        update_cask
        git -C "$HOMEBREW_TAP_DIR" -c credential.helper= -c 'credential.helper=!gh auth git-credential' push
    fi
else
    [[ -z "${HOMEBREW_TAP_DIR:-}" ]] || update_cask
    echo "Not published. To publish: git push origin main v$version && gh release create v$version $dmg"
fi
