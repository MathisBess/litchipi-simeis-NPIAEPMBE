export RUSTFLAGS="-C code-model=kernel -C codegen-units=1"

.PHONY: all build doc check test clean

all: build

build:
	@echo "Build du projet"
	cargo build --release
	@echo "Strip du binaire"
	strip target/release/simeis-server

doc:
	@echo "Creation de la doc"
	typst compile doc/manual.typ doc/manual.pdf

check:
	@echo "Check du code"
	cargo fmt -- --check
	cargo clippy -- -D warnings

test:
	@echo "Tests"
	cargo test

clean:
	@echo "Clean"
	cargo clean
	rm -f doc/manual.pdf
