# Stage 1: Build the Rust binary
FROM rust:1.79 as builder

WORKDIR /usr/src/qsvm-operator
COPY . .

# Build for release
RUN cargo build --release

# Stage 2: Create a minimal final image
FROM debian:buster-slim

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/qsvm-operator/target/release/qsvm-operator /usr/local/bin/qsvm-operator

# Set the binary as the entrypoint
ENTRYPOINT ["/usr/local/bin/qsvm-operator"]
FROM ubuntu:latest
LABEL authors="nathaniel.ham"

ENTRYPOINT ["top", "-b"]