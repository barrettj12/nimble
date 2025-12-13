use std::io::copy;

use crate::types::AgentState;
use axum::{Router, routing::get};
use axum::{
    body::Bytes,
    extract::{BodyStream, State},
    http::StatusCode,
    response::Json,
};

const PORT: u16 = 7080;

pub async fn start_api(state: AgentState) -> Result<(), Box<dyn std::error::Error>> {
    // Define routes
    let app = Router::new()
        .route("/builds", get(list_builds).post(create_build))
        .route("/builds/:id", get(get_build))
        .with_state(state);

    let addr = format!("0.0.0.0:{PORT}");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("nimbled listening on port {PORT}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn list_builds() {}

struct CreateBuildResponse {}

async fn create_build(
    State(state): State<AgentState>,
    body: Bytes,
) -> (StatusCode, Json<CreateBuildResponse>) {
    // Read tgz archive from body
    // Extract tgz to temp folder
    //
    "ok"
}

async fn get_build() {}
