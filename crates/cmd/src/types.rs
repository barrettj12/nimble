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

#[derive(Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
