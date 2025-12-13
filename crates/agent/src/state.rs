use crate::config::AgentConfig;
use crate::workers::BuildJob;
use axum::body::Bytes;
use tokio::sync::mpsc::Sender;
use tokio::{fs::File, io::AsyncWriteExt};
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentState {
    config: AgentConfig,
    pub build_queue: Sender<BuildJob>,
    // TODO: add database connection
}

impl AgentState {
    pub fn new(build_queue: Sender<BuildJob>) -> Self {
        Self {
            config: AgentConfig::new(),
            build_queue: build_queue,
        }
    }

    // Save a tgz archive containing project source code to disk
    pub async fn save_archive(
        &self,
        build_id: Uuid,
        contents: Bytes,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.config.paths().source_archive(build_id);
        let mut file = File::create(path).await?;
        file.write_all(&contents).await?;
        file.flush().await?;

        // TODO: record file info in database

        Ok(())
    }
}
