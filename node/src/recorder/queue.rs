use std::path::{Path, PathBuf};

use common::ws::SegmentPayload;
use tokio::io::AsyncWriteExt;

pub struct SegmentQueue {
    path: PathBuf,
}

impl SegmentQueue {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Appends one segment entry. Called immediately after file close — never blocks on network.
    pub async fn push(&self, entry: &SegmentPayload) -> anyhow::Result<()> {
        let mut line = serde_json::to_string(entry)?;
        line.push('\n');
        let mut f = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        f.write_all(line.as_bytes()).await?;
        Ok(())
    }

    /// Returns all pending entries in order.
    pub async fn read_all(&self) -> Vec<SegmentPayload> {
        let content = tokio::fs::read_to_string(&self.path).await.unwrap_or_default();
        content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// Removes all entries after a successful backfill flush.
    pub async fn clear(&self) {
        let _ = tokio::fs::remove_file(&self.path).await;
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
