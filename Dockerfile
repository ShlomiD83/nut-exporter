ARG APP_VERSION=0.0.0-SNAPSHOT
ARG APP_GID=5000
ARG APP_UID=5000


## Builder stage (runs natively on the build host and cross-compiles a static musl binary)
# Rust + Alpine versions are pinned in this tag so Dependabot can bump them.
FROM --platform=$BUILDPLATFORM rust:1.100.0-alpine3.24 AS builder
WORKDIR /app

# Pick the Rust target for the requested platform
ARG TARGETPLATFORM
RUN \
case $TARGETPLATFORM in \
"linux/amd64") echo x86_64-unknown-linux-musl ;; \
"linux/arm64") echo aarch64-unknown-linux-musl ;; \
"linux/arm/v7") echo armv7-unknown-linux-musleabihf ;; \
*) echo "Unsupported platform: $TARGETPLATFORM" >&2 && false ;; \
esac > /rust-target \
&& rustup target add "$(cat /rust-target)"

# Fetch deps using dummy app (cached layer)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
&& echo "fn main() {}" > src/main.rs \
&& cargo fetch --locked \
&& rm -rf src/

# Build app
COPY src/ src/
ARG APP_VERSION
RUN sed -i "s/^.*\bAPP_VERSION\b.*$/pub const APP_VERSION: \&str = \"$APP_VERSION\";/g" src/meta.rs
RUN RUST_TARGET="$(cat /rust-target)" \
&& cargo rustc --locked --target="$RUST_TARGET" --release -- -C linker=rust-lld \
&& mv "target/$RUST_TARGET/release/prometheus-nut-exporter" .


## Runtime stage (static binary only, no OS)
FROM scratch AS runtime
WORKDIR /app

ARG APP_GID
ARG APP_UID
COPY --from=builder --chown=${APP_UID}:${APP_GID} /app/prometheus-nut-exporter ./

USER ${APP_UID}:${APP_GID}
ENTRYPOINT ["./prometheus-nut-exporter"]
