use crate::api::start_api;
mod api;
mod workers;
use workers::BuildJob;
mod types;
use types::State;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create build queue
    let (build_tx, build_rx) = tokio::sync::mpsc::channel::<BuildJob>(100);

    let state = State {
        build_queue: build_tx,
    };

    start_api(state).await?;
    Ok(())
}
