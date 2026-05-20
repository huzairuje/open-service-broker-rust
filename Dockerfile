# --- builder stage ---------------------------------------------------------
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /app

# System deps needed to build sqlx with rustls + postgres.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy everything and build. Slower than a deps-cache layer but reliable.
COPY Cargo.toml ./
COPY src ./src
COPY tests ./tests
COPY catalog.example.json ./catalog.example.json
RUN cargo build --release --features postgres

# --- runtime stage ---------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home broker

WORKDIR /app
COPY --from=builder /app/target/release/rust-open-service-broker /usr/local/bin/broker
COPY catalog.example.json /app/catalog.json

USER broker
ENV BROKER_HOST=0.0.0.0 \
    BROKER_PORT=8080 \
    RUST_LOG=info

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/broker"]
