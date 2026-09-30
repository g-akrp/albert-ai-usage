#!/bin/bash
# SwiftBar plugin wrapper. This file (or a symlink to it) goes in your
# SwiftBar Plugin Folder. Refresh interval is in the filename: rename
# to change it (see the naming convention in ../providers/AGENTS.md).
#
# This is a thin wrapper on purpose: all logic lives in the Rust
# binary (core/src/swiftbar.rs), which is unit-tested; the wrapper
# only resolves paths.
set -euo pipefail

REAL="$(realpath "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "$REAL")/.." && pwd)"

export ALBERT_PROVIDERS_DIR="$ROOT/providers"
exec "$ROOT/core/target/release/albert-usage" --swiftbar
