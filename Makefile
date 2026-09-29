.PHONY: run test fmt lint check build clean

MANIFEST = core/Cargo.toml

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
