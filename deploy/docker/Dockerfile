# Multi-stage production container build for Keirox Server Daemon per KEI-REL-501
# Uses distroless static base for minimal attack surface and SLSA Level 3 compliance

FROM rust:1.78-bullseye AS builder

WORKDIR /usr/src/keirox
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin keirox-server

FROM gcr.io/distroless/cc-debian11:nonroot

WORKDIR /app
COPY --from=builder /usr/src/keirox/target/release/keirox-server /app/keirox-server
COPY config/keirox.toml /app/config/keirox.toml

USER nonroot:nonroot

EXPOSE 9092 9090 50051

ENTRYPOINT ["/app/keirox-server", "start", "--config", "/app/config/keirox.toml"]
