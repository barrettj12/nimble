use crate::state::AgentState;
use axum::http::HeaderMap;
use axum::{Router, routing::get};
use axum::{
    body::Bytes,
    extract::{BodyStream, State},
    http::StatusCode,
    response::Json,
};
use std::io::copy;
use uuid::Uuid;

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
    // TODO: check Content-Type header

    let build_id = Uuid::new_v4();

    // Body contains zipped source code - need to save this to disk
    state.save_archive(build_id, body).await?;

    // Read tgz archive from body
    // Extract tgz to temp folder
    //
    "ok"
}

async fn get_build() {}
