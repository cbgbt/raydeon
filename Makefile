.DEFAULT_GOAL := build

.PHONY: lint
lint:
	cargo clippy --locked -- -D warnings --no-deps
	uv --project pyraydeon run ruff check pyraydeon

.PHONY: check-fmt
check-fmt:
	cargo fmt --check
	uv --project pyraydeon run ruff format --check pyraydeon

.PHONY: render-test
render-test:
	./raydeon/check-examples.sh
	./pyraydeon/check-examples.sh

.PHONY: unit-test
unit-test:
	cargo test --locked

.PHONY: check
check: check-fmt lint unit-test render-test

.PHONY: build
build: check
	cargo build --locked
