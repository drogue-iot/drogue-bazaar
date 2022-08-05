.PHONY: all
all: check test

MODULES=. drogue-bazaar-core drogue-bazaar-application drogue-bazaar-actix
ARGS ?=

.PHONY: check
check:
	for i in $(MODULES); do pushd $$i; cargo check-all-features $(ARGS); popd ; done

.PHONY: test
test:
	for i in $(MODULES); do pushd $$i; cargo test $(ARGS); popd ; done

.PHONY: install
install:
	cargo install cargo-all-features
