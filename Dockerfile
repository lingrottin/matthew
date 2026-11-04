FROM rust:1.99-slim-trixie AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY lib ./lib
COPY app ./app
RUN cargo build --locked --release -p matthew-app

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates coreutils && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/matthew-app /usr/local/bin/matthew
ENV GIT_TERMINAL_PROMPT=0 GIT_LFS_SKIP_SMUDGE=1
CMD ["sleep", "infinity"]
