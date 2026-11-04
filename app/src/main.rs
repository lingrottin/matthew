mod service;
mod types;

use std::io::{Read, Write};
use types::{Input, Output, Permanent};

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .thread_stack_size(32 * 1024 * 1024)
        .build()
        .expect("runtime");
    let result = runtime.block_on(run());
    match result {
        Ok(output) => {
            let failed = output.error.is_some();
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            serde_json::to_writer(&mut lock, &output).expect("write output");
            writeln!(lock).expect("write newline");
            std::process::exit(if failed { 1 } else { 0 });
        }
        Err(error) => {
            eprintln!("Invalid input: {error}");
            std::process::exit(2);
        }
    }
}

async fn run() -> anyhow::Result<Output> {
    let mut bytes = Vec::new();
    std::io::stdin().take(65537).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 65536, "input exceeds 64 KiB");
    let input: Input = serde_json::from_slice(&bytes)?;
    let result = service::count(&input).await;
    let (data, error, retryable) = match result {
        Ok(data) => (Some(data), None, false),
        Err(error) => {
            let retryable = error.downcast_ref::<Permanent>().is_none();
            (None, Some(error.to_string()), retryable)
        }
    };
    Ok(Output {
        version: 1,
        job_id: input.job_id,
        commit_sha: input.commit_sha,
        data,
        error,
        retryable,
    })
}
