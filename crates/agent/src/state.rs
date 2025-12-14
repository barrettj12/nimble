use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::Bytes;
use tokio::{fs::File, io::AsyncWriteExt, sync::mpsc::Sender};
use uuid::Uuid;

use crate::{config::AgentConfig, workers::build::BuildJob};

#[derive(Clone)]
pub struct AgentState {
    config: Arc<AgentConfig>,
    pub build_queue: Sender<BuildJob>,
    // TODO: add database connection
}

impl AgentState {
    pub fn new(config: Arc<AgentConfig>, build_queue: Sender<BuildJob>) -> Self {
        Self {
            config,
            build_queue,
        }
    }

    // Save a tgz archive containing project source code to disk
    pub async fn save_archive(&self, build_id: Uuid, contents: Bytes) -> Result<()> {
        let path = self.config.paths().source_archive(build_id);

        // Ensure parent directory exists
        let parent = path
            .parent()
            .context("source archive path has no parent directory")?;

        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating archive directory {}", parent.display()))?;

        let mut file = File::create(&path)
            .await
            .with_context(|| format!("creating source archive {}", path.display()))?;

        file.write_all(&contents)
            .await
            .with_context(|| format!("writing source archive {}", path.display()))?;

        file.flush()
            .await
            .with_context(|| format!("flushing source archive {}", path.display()))?;

        // TODO: record file info in database

        Ok(())
    }
}
