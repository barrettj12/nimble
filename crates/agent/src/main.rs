// Module declarations
mod api;
mod config;
mod db;
mod state;
mod workers;

use std::sync::Arc;

use crate::{
    api::start_api,
    config::AgentConfig,
    db::init_pool,
    state::AgentState,
    workers::build::{BuildJob, BuildWorker},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create build queue
    let (build_tx, build_rx) = tokio::sync::mpsc::channel::<BuildJob>(100);

    let config = Arc::new(AgentConfig::new());

    // Make sure data dir exists
    let data_dir = config.get_data_dir();
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {e}"))?;

    // Ensure the db directory exists before creating the database file
    let db_path = config.paths().database();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database directory: {e}"))?;
    }

    // Initialize database
    let database_url = format!("sqlite:{}", db_path.display());
    let db = init_pool(&database_url)
        .await
        .map_err(|e| format!("Failed to initialize database: {e}"))?;

    let state = AgentState::new(Arc::clone(&config), build_tx, db.clone()).await;

    // Create and spawn build worker
    let worker = BuildWorker::new(Arc::clone(&config), db);
    tokio::spawn(async move {
        if let Err(e) = worker.run(build_rx).await {
            eprintln!("Build worker error: {e}");
        }
    });

    start_api(state).await?;
    Ok(())
}
