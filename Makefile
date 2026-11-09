WASM_TARGET := wasm32-unknown-unknown
WEB_DIR := target/volatile-web
WASM_FILE := target/$(WASM_TARGET)/release/volatile.wasm
WASM_BINDGEN ?= $(if $(wildcard target/volatile-tools/bin/wasm-bindgen),target/volatile-tools/bin/wasm-bindgen,wasm-bindgen)
PORT ?= 8000

-include .env.local
ifdef VOLATILE_SERVER_URL
export VOLATILE_SERVER_URL
endif

.PHONY: wasm-build wasm-zip wasm-run build-server

build-server:
	docker build --file server/Dockerfile --output type=local,dest=server/target/x86_64-unknown-linux-gnu/release .

wasm-build:
	cargo build --locked --release --target $(WASM_TARGET)
	rm -rf target/volatile-web
	mkdir -p $(WEB_DIR)
	$(WASM_BINDGEN) --no-typescript --target web --out-name volatile --out-dir $(WEB_DIR) $(WASM_FILE)
	cp web/index.html $(WEB_DIR)/index.html
	cp web/loading.mjs $(WEB_DIR)/loading.mjs
	printf '{"bytes":%s}\n' "$$(wc -c < $(WEB_DIR)/volatile_bg.wasm | tr -d '[:space:]')" > $(WEB_DIR)/wasm-size.json
	cp -R assets $(WEB_DIR)/assets

wasm-zip: wasm-build
	rm -f target/volatile.zip
	cd $(WEB_DIR) && zip -rq ../volatile.zip index.html loading.mjs wasm-size.json volatile.js volatile_bg.wasm assets -x '*/.DS_Store'
	mv target/volatile.zip game.zip
	@echo "Created game.zip for itch.io"

wasm-run: wasm-build
	@echo "Serving VolaTile at http://127.0.0.1:$(PORT)"
	python3 -m http.server $(PORT) --bind 127.0.0.1 --directory $(WEB_DIR)
