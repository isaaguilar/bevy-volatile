# VolaTile

For web builds, install the `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` matching the `wasm-bindgen` version in the Cargo lockfile. The build also requires `zip` and Python 3.

For the current lockfile, install the tools with `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.130 --locked`.

Run `make wasm-zip` to create an archive for itch.io. Run `make wasm-run` to build and serve the game locally; set the `PORT` variable to choose another web port.

## Local online lobby

Online play supports creating, listing, joining, and leaving rooms, optional three-digit PINs, and server-authoritative two-player matches. The host can start once both players are connected. Each player sees only their own inventory; turns pause if either player disconnects and resume after reconnection. The server allows up to seven rooms, each with a unique themed name. Rooms with no online players are hidden from the list but remain reserved for reconnection. Reconnect credentials are kept only in the open browser page, and a disconnected player's seat is reserved in memory for five minutes. If the host's reservation expires, the room closes; if the guest leaves or their reservation expires, the match ends. Reloading the page or restarting the server cannot restore a session.

Run the server locally using Cargo and the server crate's manifest. In another terminal, set `VOLATILE_SERVER_URL` to the local WebSocket endpoint and run `make wasm-run`. Open the locally served game in two browser tabs, select PLAY ONLINE, create a room in one tab, join it from the other, and start the match as host. By default, the server listens on loopback and accepts the local web client's origin.

The client server URL is a build-time setting supplied through `VOLATILE_SERVER_URL`; remote endpoints must use secure WebSockets. The server's `VOLATILE_ALLOWED_ORIGIN` accepts a comma-separated list of exact browser page origins, including their web ports. Client endpoint settings are public configuration, not secrets.

For LAN or public deployment, put a TLS-capable reverse proxy in front of the loopback-bound server and configure `VOLATILE_ALLOWED_ORIGIN` to the exact origin serving the game. Itch.io's game iframe origin must be checked on the actual deployment. Do not expose the development WebSocket directly or store secrets in the client build.

Run `make build-server` to build the x86_64 Linux release server. This requires Docker with BuildKit; the first build downloads a Rust image and Linux development packages. The binary targets Debian 12-compatible systems (glibc 2.36 or newer) and requires the `libasound2`, `libudev1`, and `libwayland-client0` runtime libraries, even though the server runs headlessly.
