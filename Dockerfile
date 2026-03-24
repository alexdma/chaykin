# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy the entire workspace (gemtext-rdf and server are workspace members)
COPY . .

# Build release binary for the server crate only
RUN cargo build --release -p chaykin

# Runtime stage
FROM alpine:latest

# Install CA certificates for HTTPS requests and libcap for setcap
RUN apk add --no-cache ca-certificates libcap

# Create non-root user
RUN addgroup -S chaykin && adduser -S chaykin -G chaykin

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/chaykin /app/chaykin

# Copy sample data
COPY server/sample_data.ttl /app/sample_data.ttl

# Grant the binary permission to bind privileged ports (< 1024, e.g. port 300)
# without running as root
RUN setcap cap_net_bind_service=+ep /app/chaykin

# Change ownership
RUN chown -R chaykin:chaykin /app

USER chaykin

# Expose all default server ports:
#    300 - Spartan
#   1900 - Nex
#   1915 - NPS
#   1965 - Gemini
EXPOSE 300 1900 1915 1965

# Default to binding on all interfaces (required for Docker)
ENTRYPOINT ["/app/chaykin"]
CMD ["--host", "0.0.0.0"]
