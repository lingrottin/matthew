# Matthew

🦀 Counts what Rust developers create.

The `lib` crate parses Rust source. The `app` crate is a single-analysis stdio CLI used by [Ferris.love](https://github.com/lingrottin/Ferris.love)'s Cloudflare Queue consumer and Containers. See [the protocol](app/API.md).

```sh
cargo test --workspace
cargo build --locked --release -p matthew-app
docker build -t matthew .
```

Pass a JSON task to `/usr/local/bin/matthew` over stdin with `docker run --rm -i --entrypoint /usr/local/bin/matthew matthew`. Containers boot with `sleep infinity` and Ferris invokes the CLI with Cloudflare's native exec API; no port is exposed. Old `Config.toml` files and HTTP tester scripts are obsolete.
