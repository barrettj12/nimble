# Build stage
FROM golang:1.21-alpine AS builder

WORKDIR /build

# Copy source code
COPY main.go .

# Build the application
RUN CGO_ENABLED=0 GOOS=linux go build -o /app/go-hello main.go

# Runtime stage
FROM alpine:latest

# Install ca-certificates for HTTPS (if needed in future)
RUN apk --no-cache add ca-certificates

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/go-hello .

# Expose the port the app runs on
EXPOSE 8080

# Run the binary
CMD ["./go-hello"]
