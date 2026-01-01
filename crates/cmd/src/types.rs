use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BuildResponse {
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateBuildResponse {
    pub build_id: String,
    pub status: String,
}

#[derive(Deserialize, Serialize)]
pub struct DeploymentResponse {
    pub id: String,
    pub build_id: String,
    pub app: String,
    pub image: String,
    pub status: String,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub address: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
