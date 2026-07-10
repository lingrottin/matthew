FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY lib/Cargo.toml ./lib/
COPY app/Cargo.toml ./app/
RUN mkdir app/src lib/src && \
    echo "fn main() {}" > app/src/main.rs && \
    echo "" > lib/src/lib.rs && \
    echo "" > lib/src/types.rs
RUN cargo build --release -p matthew-app 2>/dev/null || true

# Real build
COPY lib/src ./lib/src
COPY app/src ./app/src
RUN cargo build --release -p matthew-app

FROM alpine:3.21
RUN apk add --no-cache git ca-certificates && \
    adduser -D -h /data matthew
USER matthew
WORKDIR /data
COPY --from=builder /app/target/release/matthew-app /usr/local/bin/matthew-app
EXPOSE 3000
CMD ["matthew-app"]
