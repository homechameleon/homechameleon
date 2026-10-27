# Build context is this directory. The generator is homechameleon-core.
# syntax=docker/dockerfile:1

FROM rust:1.99.1-alpine AS server
# hadolint ignore=DL3018
RUN apk add --no-cache git musl-dev
WORKDIR /src
COPY server /src/server
COPY fetch-core.sh /fetch-core.sh
COPY fixtures/tokyo-night.colors.toml /omarchy/tests/fixtures/tokyo-night.colors.toml
RUN sh /fetch-core.sh
WORKDIR /src/server
RUN cargo build --release

FROM ghcr.io/home-assistant/base:3.24
ARG BUILD_VERSION=0.1.0
ARG BUILD_ARCH=amd64
LABEL \
    io.hass.name="HomeChameleon" \
    io.hass.description="Turn your photos into complete Home Assistant themes." \
    io.hass.arch="${BUILD_ARCH}" \
    io.hass.type="app" \
    io.hass.version="${BUILD_VERSION}" \
    org.opencontainers.image.source="https://github.com/homechameleon/homechameleon" \
    org.opencontainers.image.licenses="MIT" \
    org.opencontainers.image.title="HomeChameleon" \
    org.opencontainers.image.description="Turn your photos into complete Home Assistant themes."
COPY --from=server /src/server/target/release/homechameleon-addon /usr/bin/homechameleon-addon
COPY www /opt/homechameleon/www
COPY run.sh /etc/services.d/homechameleon/run
RUN chmod 755 /etc/services.d/homechameleon/run /usr/bin/homechameleon-addon
