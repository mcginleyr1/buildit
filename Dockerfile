# Build stage
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release -p buildit-api

# Runtime stage
FROM alpine:3.20

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/buildit-server /app/buildit-server

# Copy templates
COPY --from=builder /app/crates/buildit-api/templates /app/templates

EXPOSE 3000

ENV RUST_LOG=info

CMD ["/app/buildit-server"]
