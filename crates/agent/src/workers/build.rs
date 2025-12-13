use std::path::PathBuf;
use uuid::Uuid;

pub struct BuildJob {
    pub build_id: Uuid,
    pub source_path: PathBuf,
}
