use crate::state::AgentState;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use axum::{Router, routing::get};
use serde::Serialize;
use uuid::Uuid;

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
    pub status: String,
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

    // TODO: add build to queue

    let resp = CreateBuildResponse {
        build_id: build_id.to_string(),
        status: "queued".into(),
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
        }
    }
}
