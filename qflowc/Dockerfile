# Use the official Rust image as a builder
FROM rust:1.70 as builder

WORKDIR /usr/src/qflow-operator
COPY qflowc .

# Build the release binary
RUN cargo install --path .

# Use a minimal "distroless" image for the final container
# This improves security by removing shells and other utilities.
FROM gcr.io/distroless/cc-debian11
COPY --from=builder /usr/local/cargo/bin/qflow-operator /usr/local/bin/qflow-operator

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/qflow-operator"]