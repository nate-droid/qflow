FROM rust:1.88 as builder

# Add the MUSL target for aarch64
RUN rustup target add aarch64-unknown-linux-musl

# Set the working directory inside the container.
WORKDIR /usr/src/app

# Copy the source code into the container.
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.lock* ./

# Build the application as a static binary for the aarch64-musl target.
RUN cargo build --release --target aarch64-unknown-linux-musl

# ---- Stage 2: The Final Image ----
# Start from a 'scratch' image, which is completely empty.
# This is the most minimal and secure base for a static binary.
FROM scratch

# Copy the compiled binary from the builder stage to our final image.
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/qsim /qsim

# Set the command to run when the container starts.
CMD ["/qsim"]
