FROM rust:1.70 as builder

WORKDIR /usr/src/qflow-operator
COPY qflowc .

RUN cargo install --path .

FROM gcr.io/distroless/cc-debian11
COPY --from=builder /usr/local/cargo/bin/qflow-operator /usr/local/bin/qflow-operator

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/qflow-operator"]