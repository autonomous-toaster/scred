FROM rust:latest as builder

WORKDIR /app

# Copy only the necessary files
COPY Cargo.toml Cargo.lock ./
COPY crates crates

# Build scred-proxy in release mode
RUN cargo build --release -p scred-proxy

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/scred-proxy /usr/local/bin/scred-proxy

EXPOSE 9999

CMD ["scred-proxy"]
