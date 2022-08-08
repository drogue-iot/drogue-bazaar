.PHONY: all
all: check test

ARGS ?=

.PHONY: check
check:
	pushd drogue-bazaar-core; cargo check-all-features $(ARGS); popd
	cargo check-all-features $(ARGS)

.PHONY: test
test:
	pushd drogue-bazaar-core; cargo test $(ARGS); popd
	cargo test $(ARGS)

.PHONY: install
install:
	cargo install cargo-all-features
