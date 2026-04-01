use crate::types::ItemData;
use anyhow::anyhow;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::process::Command;
use tracing::{debug, error, info, instrument};

#[instrument(skip(client, token))]
pub async fn count(
    data_dir: PathBuf,
    repo: crate::types::Repo,
    client: Arc<reqwest::Client>,
    token: Option<String>,
) -> anyhow::Result<ItemData> {
    // 0. check data_dir
    info!(owner = %repo.owner, repo = %repo.repo, data_dir = %data_dir.display(), "count started");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }
    // 1. Filter repo size
    let query_url = format!("https://api.github.com/repos/{}/{}", repo.owner, repo.repo);

    #[derive(Deserialize, Debug)]
    struct GithubApiResponse {
        size: u64,
    }

    let mut req = client
        .get(query_url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "ferris-love-matthew");

    if let Some(ref tok) = token {
        req = req.header("Authorization", format!("Bearer {}", tok));
    }

    let repo_info = req.send().await?.json::<GithubApiResponse>().await?;
    debug!("{:?}", repo_info);
    info!(size = repo_info.size, "fetched repo size");
    if repo_info.size > 1024 * 1024 {
        error!(size = repo_info.size, "repository too large");
        return Err(anyhow!("Repository too large"));
    }

    // 2. Clone repo using git executable
    let repo_path = data_dir.join(format!("{}_{}", repo.owner, repo.repo));
    if repo_path.exists() {
        info!(path = %repo_path.display(), "removing existing repo path");
        tokio::fs::remove_dir_all(&repo_path).await?;
    }

    let log_url = format!("https://github.com/{}/{}.git", repo.owner, repo.repo);
    info!(url = %log_url, path = %repo_path.display(), authenticated = token.is_some(), "starting git clone");

    let clone_url = match token {
        Some(ref tok) => format!(
            "https://x-access-token:{}@github.com/{}/{}.git",
            tok, repo.owner, repo.repo
        ),
        None => log_url.clone(),
    };
    let output = Command::new("git")
        .arg("clone")
        .arg("--depth=1")
        .arg(&clone_url)
        .arg(&repo_path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(%stderr, "git clone failed");
        return Err(anyhow!("git clone failed: {}", stderr));
    }

    // 3. Walk repo directory iteratively and count .rs files
    #[derive(Default)]
    struct WalkStats {
        lorc: u64,
        counts: matthew::Counts,
    }
    impl std::ops::AddAssign for WalkStats {
        fn add_assign(&mut self, other: Self) {
            self.lorc += other.lorc;
            self.counts = self.counts.clone() + other.counts;
        }
    }

    fn visit_dir(root: &Path, stats: &mut WalkStats) {
        let mut pending = vec![root.to_path_buf()];

        while let Some(dir) = pending.pop() {
            let read = match std::fs::read_dir(&dir) {
                Ok(r) => r,
                Err(e) => {
                    debug!(path = %dir.display(), %e, "failed to read dir");
                    continue;
                }
            };

            for entry in read.filter_map(Result::ok) {
                let path = entry.path();
                match entry.file_type() {
                    Ok(file_type) => {
                        if file_type.is_symlink() {
                            debug!(path = %path.display(), "skipping symlink");
                            continue;
                        }

                        if file_type.is_dir() {
                            pending.push(path);
                        } else if file_type.is_file()
                            && path.extension().and_then(|s| s.to_str()) == Some("rs")
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(content) => match matthew::count_str(content.clone()) {
                                    Ok(counts) => {
                                        let file_lorc = content.lines().count() as u64;
                                        debug!(file = %path.display(), lorc = file_lorc, "counted file");
                                        *stats += WalkStats {
                                            lorc: file_lorc,
                                            counts,
                                        };
                                    }
                                    Err(_) => {
                                        debug!(file = %path.display(), "failed to count file contents");
                                    }
                                },
                                Err(e) => {
                                    debug!(file = %path.display(), %e, "failed to read file");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!(path = %path.display(), %e, "failed to stat entry");
                    }
                }
            }
        }
    }

    let mut stats = Box::new(WalkStats::default());
    visit_dir(&repo_path, &mut stats);

    let _ = tokio::fs::remove_dir_all(&repo_path).await;
    Ok(ItemData {
        lorc: stats.lorc,
        counts: stats.counts,
    })
}
