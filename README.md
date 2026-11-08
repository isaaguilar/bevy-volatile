# VolaTile

For web builds, install the `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` matching the `wasm-bindgen` version in `Cargo.lock`. The build also requires `zip` and Python 3.

For the current lockfile, install the tools locally with `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.130 --locked --root target/volatile-tools`. The Makefile uses that local installation if present, or `wasm-bindgen` on `PATH` otherwise.

Run `make wasm-zip` to create `game.zip` for itch.io. Run `make wasm-run` to build and serve the game at `http://localhost:8000`; use `make wasm-run PORT=8001` to choose another port.
