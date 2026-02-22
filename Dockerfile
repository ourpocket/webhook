FROM rust:slim-trixie AS builder

WORKDIR /app

# Copy manifests and code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates


RUN cargo build --release -p webhook-server

FROM debian:trixie-slim

RUN useradd -m appuser


COPY --from=builder /app/target/release/webhook-server /usr/local/bin/webhook-app

RUN chmod +x /usr/local/bin/webhook-app && chown appuser:appuser /usr/local/bin/webhook-app

USER appuser
ENV RUST_LOG=info


ENTRYPOINT ["/usr/local/bin/webhook-app"]