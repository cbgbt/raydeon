.DEFAULT_GOAL := build

.PHONY: lint
lint:
	cargo clippy --locked -- -D warnings --no-deps
	uv --project pyraydeon run ruff check pyraydeon

.PHONY: check-fmt
check-fmt:
	cargo fmt --check
	uv --project pyraydeon run ruff format --check pyraydeon

.PHONY: rust-render-test
rust-render-test:
	./raydeon/check-examples.sh

.PHONY: reinstall-py-venv
reinstall-py-venv:
	echo "Reinstalling native dependencies in virtualenv..."
	uv --project pyraydeon run --reinstall python -c 'print("Reinstalled dependencies")'


.PHONY: py-render-test
py-render-test: reinstall-py-venv
	./pyraydeon/check-examples.sh

.PHONY: render-test
render-test: rust-render-test py-render-test

.PHONY: unit-test
unit-test:
	cargo test --locked

.PHONY: check
check: check-fmt lint unit-test render-test

.PHONY: build
build: check
	cargo build --locked
