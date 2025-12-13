// Module declarations
mod api;
mod config;
mod state;
mod workers;

use crate::api::start_api;
use crate::state::AgentState;
use crate::workers::BuildJob;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create build queue
    let (build_tx, _build_rx) = tokio::sync::mpsc::channel::<BuildJob>(100);

    let state = AgentState::new(build_tx);

    // TODO: need to pass build_rx to build worker

    start_api(state).await?;
    Ok(())
}
