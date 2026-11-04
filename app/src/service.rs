use crate::types::{Data, Input, reject};
use anyhow::{Context, Result};
use base64::Engine;
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::{path::Path, time::Duration};
use tokio::process::Command;

const MAX_SNAPSHOT_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_RUST_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RUST_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_FILES: u64 = 100_000;
const MAX_TREE_REQUESTS: usize = 1_000;

#[derive(Deserialize)]
struct Tree {
    tree: Vec<Entry>,
    truncated: bool,
}
#[derive(Deserialize)]
struct Entry {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
    mode: String,
}
#[derive(Deserialize)]
struct Commit {
    tree: TreeRef,
}
#[derive(Deserialize)]
struct TreeRef {
    sha: String,
}

#[derive(Default)]
struct Budget {
    bytes: u64,
    rust_bytes: u64,
    files: u64,
}
impl Budget {
    fn add(&mut self, entry: &Entry) -> Result<()> {
        if entry.kind != "blob" {
            return Ok(());
        }
        let size = entry
            .size
            .ok_or_else(|| reject("GitHub tree omitted a blob size"))?;
        self.bytes = self
            .bytes
            .checked_add(size)
            .ok_or_else(|| reject("Snapshot size overflow"))?;
        self.files += 1;
        if self.bytes > MAX_SNAPSHOT_BYTES || self.files > MAX_FILES {
            return Err(reject("HEAD snapshot exceeds 1 GiB or 100,000 files"));
        }
        if entry.path.ends_with(".rs") && entry.mode != "120000" {
            self.rust_bytes += size;
            if size > MAX_RUST_FILE_BYTES || self.rust_bytes > MAX_RUST_BYTES {
                return Err(reject("Rust input exceeds 8 MiB per file or 256 MiB total"));
            }
        }
        Ok(())
    }
}

async fn get<T: DeserializeOwned>(client: &Client, input: &Input, suffix: &str) -> Result<T> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/{}",
        input.owner, input.repo, suffix
    );
    let mut req = client
        .get(url)
        .header("Accept", "application/vnd.github+json");
    if let Some(token) = &input.token {
        req = req.bearer_auth(token);
    }
    let response = req.send().await?;
    let code = response.status().as_u16();
    if matches!(code, 404 | 409 | 422) {
        return Err(reject(format!(
            "GitHub returned {code}; repository or commit unavailable"
        )));
    }
    // Do not emit GitHub response bodies or credential-bearing request URLs.
    if !response.status().is_success() {
        anyhow::bail!("GitHub returned {code}");
    }
    Ok(response.json().await?)
}

async fn snapshot_budget(client: &Client, input: &Input) -> Result<Budget> {
    let commit: Commit = get(client, input, &format!("git/commits/{}", input.commit_sha)).await?;
    let tree: Tree = get(
        client,
        input,
        &format!("git/trees/{}?recursive=1", commit.tree.sha),
    )
    .await?;
    let mut budget = Budget::default();
    if !tree.truncated {
        for entry in &tree.tree {
            budget.add(entry)?;
        }
        return Ok(budget);
    }
    // A recursive response is not paginated. Restart from the root; partial sums are discarded.
    let mut pending = vec![commit.tree.sha];
    let mut requests = 0;
    while let Some(sha) = pending.pop() {
        requests += 1;
        if requests > MAX_TREE_REQUESTS {
            return Err(reject("HEAD tree requires more than 1,000 API requests"));
        }
        let tree: Tree = get(client, input, &format!("git/trees/{sha}")).await?;
        if tree.truncated {
            return Err(reject(
                "GitHub truncated a non-recursive tree; cannot verify snapshot size",
            ));
        }
        for entry in tree.tree {
            if entry.kind == "tree" {
                pending.push(entry.sha.clone());
            }
            budget.add(&entry)?;
        }
    }
    Ok(budget)
}

fn validate(input: &Input) -> Result<()> {
    let valid_name = |value: &str| {
        !value.is_empty()
            && value.len() <= 100
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            && value != "."
            && value != ".."
    };
    if !valid_name(&input.owner)
        || !valid_name(&input.repo)
        || input.commit_sha.len() != 40
        || !input.commit_sha.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(reject("Invalid owner, repository, or full commit SHA"));
    }
    Ok(())
}

async fn git(input: &Input, cwd: &Path, args: &[&str]) -> Result<()> {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(cwd)
        .kill_on_drop(true)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    if let Some(token) = &input.token {
        let credential =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        command
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "http.https://github.com/.extraheader")
            .env(
                "GIT_CONFIG_VALUE_0",
                format!("Authorization: Basic {credential}"),
            );
    }
    let output = command.output().await.context("start git")?;
    if !output.status.success() {
        // Keep tokens, raw remote output and argv out of diagnostics.
        anyhow::bail!(
            "git {} failed ({})",
            args.first().unwrap_or(&"command"),
            output.status
        );
    }
    Ok(())
}

pub async fn count(input: &Input) -> Result<Data> {
    validate(input)?;
    let client = Client::builder()
        .user_agent("ferris-love-matthew")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let budget = snapshot_budget(&client, input).await?;
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    git(input, root, &["init", "--quiet"]).await?;
    let remote = format!("https://github.com/{}/{}.git", input.owner, input.repo);
    git(input, root, &["remote", "add", "origin", &remote]).await?;
    git(
        input,
        root,
        &[
            "fetch",
            "--quiet",
            "--depth=1",
            "--no-tags",
            "origin",
            &input.commit_sha,
        ],
    )
    .await?;
    git(
        input,
        root,
        &[
            "-c",
            "core.hooksPath=/dev/null",
            "checkout",
            "--quiet",
            "--detach",
            "FETCH_HEAD",
        ],
    )
    .await?;
    let scan_root = root.to_path_buf();
    let mut data = tokio::task::spawn_blocking(move || scan(&scan_root)).await??;
    data.snapshot_bytes = budget.bytes;
    Ok(data)
}

fn scan(root: &Path) -> Result<Data> {
    let mut data = Data::default();
    let mut pending = vec![root.to_path_buf()];
    let mut files = 0;
    let mut bytes = 0;
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ty = entry.file_type()?;
            if ty.is_symlink() || entry.file_name() == ".git" {
                continue;
            }
            if ty.is_dir() {
                pending.push(path);
                continue;
            }
            if !ty.is_file() {
                continue;
            }
            files += 1;
            if files > MAX_FILES {
                return Err(reject("Checkout exceeds file limit"));
            }
            if path.extension().and_then(|v| v.to_str()) != Some("rs") {
                continue;
            }
            let size = entry.metadata()?.len();
            bytes += size;
            if size > MAX_RUST_FILE_BYTES || bytes > MAX_RUST_BYTES {
                return Err(reject("Checkout exceeds Rust input limit"));
            }
            let content = match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                    data.skipped_files += 1;
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let lines = content.lines().count() as u64;
            match matthew::count_str(content) {
                Ok(counts) => {
                    data.lorc += lines;
                    data.counts = data.counts + counts;
                    data.parsed_files += 1;
                }
                Err(_) => data.skipped_files += 1,
            }
        }
    }
    if data.parsed_files == 0 && data.skipped_files > 0 {
        return Err(reject("No Rust files could be parsed"));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(path: &str, size: u64) -> Entry {
        Entry {
            path: path.into(),
            kind: "blob".into(),
            sha: "a".repeat(40),
            size: Some(size),
            mode: "100644".into(),
        }
    }
    #[test]
    fn uses_bytes_and_counts_each_path() {
        let mut b = Budget::default();
        b.add(&entry("a.rs", 12)).unwrap();
        b.add(&entry("b.rs", 12)).unwrap();
        assert_eq!((b.bytes, b.rust_bytes, b.files), (24, 24, 2));
        assert!(b.add(&entry("huge.bin", MAX_SNAPSHOT_BYTES)).is_err());
    }
    #[test]
    fn rejects_oversized_rust_file() {
        assert!(
            Budget::default()
                .add(&entry("a.rs", MAX_RUST_FILE_BYTES + 1))
                .is_err()
        );
    }
    #[test]
    fn skips_git_symlinks_and_reports_parse_failures() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join(".git")).unwrap();
        std::fs::write(d.path().join(".git/hidden.rs"), "fn hidden() {}").unwrap();
        std::fs::write(d.path().join("valid.rs"), "fn f() {}\n").unwrap();
        std::fs::write(d.path().join("bad.rs"), "fn {").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("valid.rs", d.path().join("link.rs")).unwrap();
        let result = scan(d.path()).unwrap();
        assert_eq!(
            (
                result.lorc,
                result.parsed_files,
                result.skipped_files,
                result.counts.def_fn
            ),
            (1, 1, 1, 1)
        );
    }
}
