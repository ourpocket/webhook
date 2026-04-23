FROM rust:slim-trixie AS builder

WORKDIR /app

# Build argument for binary name
ARG BIN_NAME=webhook-server

# Copy manifests and code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates


RUN cargo build --release -p ${BIN_NAME}

FROM debian:trixie-slim

RUN useradd -m appuser


COPY --from=builder /app/target/release/${BIN_NAME} /usr/local/bin/app

RUN chmod +x /usr/local/bin/app && chown appuser:appuser /usr/local/bin/app

USER appuser
ENV RUST_LOG=info


ENTRYPOINT ["/usr/local/bin/app"]