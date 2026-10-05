# Version number

Use semantic versioning: `MAJOR.MINOR.PATCH`. The current version is 1.3.0.

- Never edit version numbers by hand. `scripts/release.sh` sets `CFBundleShortVersionString` and increments `CFBundleVersion` in `Resources/Info.plist`, and sets `AppVersion.current`.
- The release commit is `release: <version>` with a tag for it.
