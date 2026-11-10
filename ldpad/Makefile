.PHONY: build test bench reproduce tables figures verify-rust clean fmt clippy

build:
	cargo build --release --locked

test:
	cargo test --locked --all-features

bench:
	cargo bench --locked

# The reproducibility pipeline, inputs and paper live in the package parent.
reproduce:
	$(MAKE) -C .. reproduce

tables:
	$(MAKE) -C .. tables

figures:
	$(MAKE) -C .. figures

verify-rust:
	$(MAKE) -C .. verify-rust

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
