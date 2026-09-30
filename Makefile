.PHONY: run test fmt lint check build clean swiftbar-install icon-preview

MANIFEST = core/Cargo.toml

# Runs every providers/*.json config live -- see providers/AGENTS.md
# (aka example/AGENTS.md) for the config format. Set
# ALBERT_PROVIDERS_DIR to point at a different config folder.
run:
	cargo run --manifest-path $(MANIFEST) --quiet

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

# Builds the release binary, then symlinks swiftbar/albert-usage.30s.sh
# into SwiftBar's own configured Plugin Folder (its PluginDirectory
# preference, com.ameba.SwiftBar -- read from SwiftBar itself, not
# guessed or hardcoded). Re-run after moving/renaming the repo.
swiftbar-install: build
	@dir=$$(defaults read com.ameba.SwiftBar PluginDirectory 2>/dev/null); \
	if [ -z "$$dir" ]; then \
		echo "SwiftBar's Plugin Folder isn't set. Open SwiftBar > Preferences and set one, then re-run this."; \
		exit 1; \
	fi; \
	mkdir -p "$$dir"; \
	ln -sf "$$(pwd)/swiftbar/albert-usage.30s.sh" "$$dir/"; \
	echo "Linked into $$dir -- refresh SwiftBar (or wait for the 30s interval)."

# Renders each live provider's real icon at a large, easy-to-inspect
# scale (not the small 2x used in the actual menu bar) and opens the
# result -- a repeatable way to visually check the icon looks right
# without squinting at the real status bar.
icon-preview: build
	@cargo run --manifest-path $(MANIFEST) --quiet -- --icon-preview
	@open /tmp/albert-icon-preview/*.png 2>/dev/null || true
