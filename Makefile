WASM_TARGET := wasm32-unknown-unknown
WEB_DIR := target/volatile-web
WASM_FILE := target/$(WASM_TARGET)/release/volatile.wasm
WASM_BINDGEN ?= $(if $(wildcard target/volatile-tools/bin/wasm-bindgen),target/volatile-tools/bin/wasm-bindgen,wasm-bindgen)
PORT ?= 8000

.PHONY: wasm-build wasm-zip wasm-run

wasm-build:
	cargo build --locked --release --target $(WASM_TARGET)
	rm -rf target/volatile-web
	mkdir -p $(WEB_DIR)
	$(WASM_BINDGEN) --no-typescript --target web --out-name volatile --out-dir $(WEB_DIR) $(WASM_FILE)
	cp web/index.html $(WEB_DIR)/index.html
	cp -R assets $(WEB_DIR)/assets

wasm-zip: wasm-build
	rm -f target/volatile.zip
	cd $(WEB_DIR) && zip -rq ../volatile.zip index.html volatile.js volatile_bg.wasm assets -x '*/.DS_Store'
	mv target/volatile.zip game.zip
	@echo "Created game.zip for itch.io"

wasm-run: wasm-build
	@echo "Serving VolaTile at http://localhost:$(PORT)"
	python3 -m http.server $(PORT) --bind 127.0.0.1 --directory $(WEB_DIR)
