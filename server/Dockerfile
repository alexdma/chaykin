# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy the server source
COPY . .

# Build release binary
RUN cargo build --release

# Runtime stage
FROM alpine:latest

# Install CA certificates for HTTPS requests
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN addgroup -S chaykin && adduser -S chaykin -G chaykin

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/chaykin /app/chaykin

# Copy sample data (assuming it's in the server directory)
COPY sample_data.ttl /app/sample_data.ttl

# Change ownership
RUN chown -R chaykin:chaykin /app

USER chaykin

# Expose Gemini port
EXPOSE 1965

# Default to binding on all interfaces (required for Docker)
ENTRYPOINT ["/app/chaykin"]
CMD ["--host", "0.0.0.0"]
