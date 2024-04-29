WASM_TARGET = wasm32-wasi

.PHONY: all
all: github_accept_webhook.wasm wasm_docker.wasm

%.wasm: ./$(basename $@)/src/*.rs
	cargo build --package $(basename $@) --release --target $(WASM_TARGET)

github_accept_webhook.wasm:
wasm_docker.wasm:
