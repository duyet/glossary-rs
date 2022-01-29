# Build stage
FROM rust:latest AS builder

RUN update-ca-certificates
WORKDIR /app

# Download crates-io index and fetch dependency code.
# This step avoids needing to spend time on every build downloading the index
# which can take a long time within the docker context. Docker will cache it.
RUN USER=root cargo init
COPY Cargo.toml Cargo.toml
RUN cargo fetch

# Copy and compile
COPY src src
RUN cargo build --release

# Run stage
FROM debian:buster-slim
WORKDIR /app
RUN apt-get update && apt-get install libpq5 -y
ENV DATABASE_URL postgres://postgres:5432/postgres

# Copy from build stage
COPY --from=builder \
     /app/target/release/insights_glossary \
    api
EXPOSE 8080
CMD ["/app/api"]
