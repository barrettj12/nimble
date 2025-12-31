use std::str::FromStr;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::error::TrySendError;
use uuid::Uuid;

use crate::{
    db::{self, DeploymentRecord},
    state::ApiState,
    workers::{
        build::{BuildJob, BuildStatus},
        deploy::DeployStatus,
    },
};

// TODO: move this into AgentConfig
const PORT: u16 = 7080;

pub async fn start_api(state: ApiState) -> Result<(), Box<dyn std::error::Error>> {
    // Define routes
    let app = Router::new()
        .route("/builds", get(list_builds).post(create_build))
        .route("/builds/:id", get(get_build))
        .route("/deployments", get(list_deployments))
        .route("/deployments/:id", get(get_deployment))
        .with_state(state);

    let addr = format!("0.0.0.0:{PORT}");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("nimbled listening on port {PORT}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Deserialize)]
struct ListBuildsQuery {
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct BuildResponse {
    id: String,
    status: BuildStatus,
    created_at: String,
    updated_at: String,
}

impl From<db::BuildRecord> for BuildResponse {
    fn from(record: db::BuildRecord) -> Self {
        BuildResponse {
            id: record.id.to_string(),
            status: record.status,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

async fn list_builds(
    State(state): State<ApiState>,
    Query(params): Query<ListBuildsQuery>,
) -> Result<Json<Vec<BuildResponse>>, ApiError> {
    // Parse status filter if provided
    let status_filter = if let Some(status_str) = params.status {
        Some(
            BuildStatus::from_str(&status_str)
                .map_err(|e| ApiError::BadRequest(format!("Invalid status: {e}")))?,
        )
    } else {
        None
    };

    let builds = state
        .db
        .list_builds(params.limit, status_filter)
        .await
        .map_err(ApiError::Internal)?;

    let responses: Vec<BuildResponse> = builds.into_iter().map(BuildResponse::from).collect();
    Ok(Json(responses))
}

#[derive(Serialize)]
struct CreateBuildResponse {
    build_id: String,
    status: BuildStatus,
}

#[derive(Deserialize, Default)]
struct CreateBuildQuery {
    deploy: Option<bool>,
}

async fn create_build(
    State(state): State<ApiState>,
    Query(params): Query<CreateBuildQuery>,
    body: Bytes,
) -> Result<Json<CreateBuildResponse>, ApiError> {
    // TODO: check Content-Type header

    let build_id = Uuid::new_v4();
    let deploy = params.deploy.unwrap_or(true);

    // Body contains zipped source code - need to save this to disk
    // TODO: we should check early that the file format is correct (tgz)
    // and return BadRequest if not.
    state
        .save_archive(build_id, body)
        .await
        .map_err(ApiError::Internal)?;

    // Add build to queue
    let job = BuildJob { build_id, deploy };
    state.build_queue.try_send(job).map_err(|e| match e {
        TrySendError::Full(_) => {
            ApiError::ServiceUnavailable("build queue is full, please try again later".to_string())
        }
        TrySendError::Closed(_) => ApiError::Internal(anyhow::anyhow!("build queue is closed")),
    })?;

    // Record build in database as queued
    state
        .db
        .create_build(build_id, BuildStatus::Queued)
        .await
        .map_err(ApiError::Internal)?;

    let resp = CreateBuildResponse {
        build_id: build_id.to_string(),
        status: BuildStatus::Queued,
    };
    Ok(Json(resp))
}

async fn get_build(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<BuildResponse>, ApiError> {
    let build_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest(format!("Invalid build ID: {id}")))?;

    let build = state
        .db
        .get_build(build_id)
        .await
        .map_err(ApiError::Internal)?;

    match build {
        Some(record) => Ok(Json(BuildResponse::from(record))),
        None => Err(ApiError::NotFound),
    }
}

#[derive(Deserialize)]
struct ListDeploymentsQuery {
    build_id: Option<String>,
}

#[derive(Serialize)]
struct DeploymentResponse {
    id: String,
    build_id: String,
    image: String,
    status: DeployStatus,
    container_id: Option<String>,
    container_name: Option<String>,
    address: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<DeploymentRecord> for DeploymentResponse {
    fn from(record: DeploymentRecord) -> Self {
        DeploymentResponse {
            id: record.id.to_string(),
            build_id: record.build_id.to_string(),
            image: record.image,
            status: record.status,
            container_id: record.container_id,
            container_name: record.container_name,
            address: record.address,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

async fn list_deployments(
    State(state): State<ApiState>,
    Query(params): Query<ListDeploymentsQuery>,
) -> Result<Json<Vec<DeploymentResponse>>, ApiError> {
    let build_filter = if let Some(build_id) = params.build_id {
        Some(
            Uuid::parse_str(&build_id)
                .map_err(|_| ApiError::BadRequest(format!("Invalid build ID: {build_id}")))?,
        )
    } else {
        None
    };

    let deployments = state
        .db
        .list_deployments(build_filter)
        .await
        .map_err(ApiError::Internal)?;

    Ok(Json(
        deployments
            .into_iter()
            .map(DeploymentResponse::from)
            .collect(),
    ))
}

async fn get_deployment(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<DeploymentResponse>, ApiError> {
    let deploy_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest(format!("Invalid deployment ID: {id}")))?;

    let deployment = state
        .db
        .get_deployment(deploy_id)
        .await
        .map_err(ApiError::Internal)?;

    match deployment {
        Some(record) => Ok(Json(DeploymentResponse::from(record))),
        None => Err(ApiError::NotFound),
    }
}

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
