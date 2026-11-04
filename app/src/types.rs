use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub job_id: String,
    pub owner: String,
    pub repo: String,
    pub commit_sha: String,
    pub token: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub lorc: u64,
    pub counts: matthew::Counts,
    pub parsed_files: u64,
    pub skipped_files: u64,
    pub snapshot_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub version: u8,
    pub job_id: String,
    pub commit_sha: String,
    pub data: Option<Data>,
    pub error: Option<String>,
    pub retryable: bool,
}

#[derive(Debug)]
pub struct Permanent(pub String);
impl std::fmt::Display for Permanent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Permanent {}
pub fn reject(message: impl Into<String>) -> anyhow::Error {
    Permanent(message.into()).into()
}
