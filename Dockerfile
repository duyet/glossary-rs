# Build stage
FROM rust:1-buster AS builder

RUN update-ca-certificates
WORKDIR /app

# Copy and compile
COPY . .
RUN cargo build --release

# Run stage
FROM debian:buster-slim
WORKDIR /app
RUN apt-get update && apt-get install libpq5 -y
ENV DATABASE_URL postgres://postgres:5432/postgres

# Copy from build stage
COPY --from=builder /app/target/release/glossary api
EXPOSE 8080
CMD ["/app/api"]
