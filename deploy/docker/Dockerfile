# Use standard rust image for builder
FROM rust:1.80-bullseye AS builder

WORKDIR /usr/src/keirox

# Copy the entire workspace
COPY . .

# Build the release binary
RUN cargo build --release --bin keirox-server

# Use a distroless base image for runtime security (nonroot)
FROM gcr.io/distroless/cc-debian11:nonroot

# Copy the compiled binary
COPY --from=builder /usr/src/keirox/target/release/keirox-server /usr/local/bin/keirox-server

# Expose ports
EXPOSE 9092 50051 9090 9091

# Run as nonroot
USER nonroot

# Use O_DIRECT and io_uring, volume should be mounted at /var/lib/keirox/data
VOLUME ["/var/lib/keirox/data"]

ENTRYPOINT ["/usr/local/bin/keirox-server"]
CMD ["start"]
