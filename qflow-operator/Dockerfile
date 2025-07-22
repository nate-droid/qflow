FROM rust:1.88 as builder

RUN apt-get update && apt-get install -y musl-tools gcc-aarch64-linux-gnu

RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/qflow-operator
COPY . .

RUN cargo build --target aarch64-unknown-linux-musl --release

FROM gcr.io/distroless/static-debian11
COPY --from=builder /usr/src/qflow-operator/target/aarch64-unknown-linux-musl/release/qflow-operator /usr/local/bin/qflow-operator