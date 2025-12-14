// Module declarations
mod api;
mod config;
mod state;
mod workers;

use std::sync::Arc;

use crate::{
    api::start_api,
    config::AgentConfig,
    state::AgentState,
    workers::build::{BuildJob, BuildWorker},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create build queue
    let (build_tx, build_rx) = tokio::sync::mpsc::channel::<BuildJob>(100);

    let config = Arc::new(AgentConfig::new());
    let state = AgentState::new(Arc::clone(&config), build_tx);

    // Create and spawn build worker
    let worker = BuildWorker::new(Arc::clone(&config));
    tokio::spawn(async move {
        if let Err(e) = worker.run(build_rx).await {
            eprintln!("Build worker error: {e}");
        }
    });

    start_api(state).await?;
    Ok(())
}
