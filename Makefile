.DEFAULT_GOAL := build

# A cached pyraydeon wheel can mask rust changes entirely; always rebuild.
export UV_NO_CACHE = 1

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

.PHONY: file-length
file-length:
	git ls-files '*.rs' | xargs wc -l | awk '$$2 != "total" && $$1 > 600 {print; bad=1} END {exit bad}'

.PHONY: check
check: check-fmt lint unit-test render-test file-length

.PHONY: build
build: check
	cargo build --locked
