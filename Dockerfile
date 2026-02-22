FROM rust:1.82-slim AS builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/target/release/webhook /usr/local/bin/webhook

ENV PORT=5000
EXPOSE 5000

CMD ["webhook"]
