use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tokio::sync::mpsc::error::TrySendError;
use uuid::Uuid;

use crate::{
    state::AgentState,
    workers::build::{BuildJob, BuildStatus},
};

// TODO: move this into AgentConfig
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

#[derive(Serialize)]
struct CreateBuildResponse {
    build_id: String,
    status: BuildStatus,
}

async fn create_build(
    State(state): State<AgentState>,
    body: Bytes,
) -> Result<Json<CreateBuildResponse>, ApiError> {
    // TODO: check Content-Type header

    let build_id = Uuid::new_v4();

    // Body contains zipped source code - need to save this to disk
    state
        .save_archive(build_id, body)
        .await
        .map_err(ApiError::Internal)?;

    // Add build to queue
    let job = BuildJob { build_id };
    state.build_queue.try_send(job).map_err(|e| match e {
        TrySendError::Full(_) => {
            ApiError::ServiceUnavailable("build queue is full, please try again later".to_string())
        }
        TrySendError::Closed(_) => ApiError::Internal(anyhow::anyhow!("build queue is closed")),
    })?;

    let resp = CreateBuildResponse {
        build_id: build_id.to_string(),
        status: BuildStatus::Queued,
    };
    Ok(Json(resp))
}

async fn get_build() {}

// Errors

// ApiError represents errors returned by the API.
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    Internal(anyhow::Error),
    ServiceUnavailable(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "not found".into(),
                }),
            )
                .into_response(),

            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: msg })).into_response()
            }

            ApiError::Internal(err) => {
                tracing::error!(?err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal server error".into(),
                    }),
                )
                    .into_response()
            }

            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse { error: msg }),
            )
                .into_response(),
        }
    }
}
