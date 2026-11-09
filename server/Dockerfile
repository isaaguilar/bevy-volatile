FROM rust:1-bookworm AS build

RUN dpkg --add-architecture amd64 \
    && apt-get update \
    && apt-get install --assume-yes --no-install-recommends \
        gcc-x86-64-linux-gnu libc6-dev-amd64-cross libasound2-dev:amd64 libudev-dev:amd64 \
        libwayland-dev:amd64 libxkbcommon-dev:amd64 pkg-config \
    && rustup target add x86_64-unknown-linux-gnu

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_LIBDIR=/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig

WORKDIR /src
COPY Cargo.toml ./
COPY src/ src/
COPY assets/ assets/
COPY game-rules/Cargo.toml game-rules/Cargo.toml
COPY game-rules/src/ game-rules/src/
COPY online-protocol/Cargo.toml online-protocol/Cargo.toml
COPY online-protocol/src/ online-protocol/src/
COPY server/Cargo.toml server/Cargo.lock server/
COPY server/src/ server/src/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/server/target \
    cargo build --locked --release --manifest-path server/Cargo.toml --target x86_64-unknown-linux-gnu \
    && cp server/target/x86_64-unknown-linux-gnu/release/volatile-server /volatile-server

FROM scratch
COPY --from=build /volatile-server /volatile-server
