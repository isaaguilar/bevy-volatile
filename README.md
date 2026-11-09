# VolaTile

For web builds, install the `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` matching the `wasm-bindgen` version in the Cargo lockfile. The build also requires `zip` and Python 3.

For the current lockfile, install the tools with `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.130 --locked`.

Run `make wasm-zip` to create an archive for itch.io. Run `make wasm-run` to build and serve the game locally; set the `PORT` variable to choose another web port.

