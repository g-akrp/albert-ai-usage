# Release

Release through GitHub Release. The repository is private, so users need access.

## Process

```
scripts/release.sh 1.1.0            # build locally: version, checks, DMG, commit, tag
scripts/release.sh 1.1.0 --publish  # also push main and the tag, and create the GitHub release
```

- Runs only on `main` without uncommitted changes to tracked files.
- Sets the version (see [version number](version-number.md)), runs `swift run CoreChecks`, builds the app, and writes `build/AIUsage-<version>.dmg` with a `.sha256` file.
- If anything fails before the release commit, the version edits are undone.
- Publishing needs the `gh` CLI logged in and an `origin` remote.
- Commit, push, and publish happen only when the maintainer asks.

## Artifact

- `AIUsage-<version>.dmg`: drag **AI Usage** onto **Applications**.
- Signed ad hoc, not with an Apple Developer ID. First launch: System Settings › Privacy & Security › **Open Anyway**.
- Verify with `shasum -a 256 -c AIUsage-<version>.dmg.sha256`.

## Update

Quit the app, install the new DMG over the old app, and open it. The installed version is at the bottom of the panel.
