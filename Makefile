.PHONY: run test fmt lint check build clean run-live run-live-copilot run-live-all

MANIFEST = core/Cargo.toml

run:
	cargo run --manifest-path $(MANIFEST) --quiet

# Off by default (see core/src/process.rs). Spawns the real local
# `codex app-server` instead of reading the fixture. Requires `codex`
# on PATH; no credentials or auth-state files are read.
run-live:
	ALBERT_LIVE_CODEX=1 cargo run --manifest-path $(MANIFEST) --quiet

# Off by default. Calls `gh api copilot_internal/user`. Requires `gh`
# on PATH, logged in; gh handles its own token, nothing here reads a
# credential file. See docs/data-source/copilot.md for the stale-env-
# var gotcha (GITHUB_TOKEN/GH_TOKEN can shadow a working login).
run-live-copilot:
	ALBERT_LIVE_COPILOT=1 cargo run --manifest-path $(MANIFEST) --quiet

run-live-all:
	ALBERT_LIVE_CODEX=1 ALBERT_LIVE_COPILOT=1 cargo run --manifest-path $(MANIFEST) --quiet

test:
	cargo test --manifest-path $(MANIFEST)

fmt:
	cargo fmt --manifest-path $(MANIFEST)

lint:
	cargo clippy --manifest-path $(MANIFEST) --all-targets -- -D warnings

check: fmt lint test

build:
	cargo build --manifest-path $(MANIFEST) --release

clean:
	cargo clean --manifest-path $(MANIFEST)
