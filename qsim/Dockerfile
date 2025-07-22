FROM rust:1.88 as builder

# Add the MUSL target for aarch64
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/app

COPY ./src ./src
COPY ./Cargo.toml ./Cargo.lock* ./

RUN cargo build --release --target aarch64-unknown-linux-musl

FROM scratch

COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/qsim /qsim

CMD ["/qsim"]
