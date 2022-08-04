.PHONY: all
all: check test

.PHONY: check
check:
	cargo check-all-features

.PHONY: test
test:
	cargo test-all-features

.PHONY: install
install:
	cargo install cargo-all-features
