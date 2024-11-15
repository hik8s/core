FROM rust:1.82 AS builder

RUN apt-get update && apt-get install -y lld clang ca-certificates protobuf-compiler

COPY ./rs/ /rs/
COPY ./Cargo.toml /Cargo.toml

RUN cargo build --release

# data-intake
FROM debian:bookworm-slim AS data-intake
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /target/release/data-intake /data-intake
CMD ["/data-intake"]

# data-processing
FROM debian:bookworm-slim AS data-processing
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /target/release/data-processing /data-processing
CMD ["/data-processing"]

# data-vectorizer
FROM debian:bookworm-slim AS data-vectorizer
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /target/release/data-vectorizer /data-vectorizer
CMD ["/data-vectorizer"]

# chat-backend
FROM debian:bookworm-slim AS chat-backend
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /target/release/chat-backend /chat-backend
CMD ["/chat-backend"]
