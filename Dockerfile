# Build stage
FROM rust:1.73.0 AS builder
WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/zero2prod zero2prod
# We need the configuration file at runtime!
COPY configuration.yaml configuration.yaml
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./target/release/zero2prod"]


