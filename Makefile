.PHONY: all
all: check test

ARGS ?=

.PHONY: check
check:
	cargo check-all-features $(ARGS)

.PHONY: test
test:
	cargo test $(ARGS)

.PHONY: install
install:
	cargo install cargo-all-features
