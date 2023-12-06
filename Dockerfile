################################################################################
# Create a stage for building the application.

ARG RUST_VERSION=1.74.0
ARG APP_NAME=zero2prod
FROM rust:${RUST_VERSION}-slim-bookworm AS build
ARG APP_NAME
WORKDIR /app

# RUN apt update && apt install -y pkg-config libssl-dev gcc-x86-64-linux-gnu && \
    # rustup target add x86_64-unknown-linux-gnu

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=templates,target=templates \
    --mount=type=bind,source=.cargo,target=.cargo \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    --mount=type=bind,source=migrations,target=migrations \
    set -e && \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /app/$APP_NAME

################################################################################
# Create a stage for running the application.
FROM debian:bookworm-slim AS final

# Create a non-privileged user that the app will run under.
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser


WORKDIR /app
# Copy the executable from the "build" stage.
COPY --from=build /app/$APP_NAME /app/$APP_NAME
COPY configuration/production.yaml ./configuration/production.yaml
COPY migrations ./migrations
COPY templates ./templates
ENV APP_ENVIRONMENT production

# Expose the port that the application listens on.
EXPOSE 8000

# What the container should run when it is started.
CMD ["/app/zero2prod"]

