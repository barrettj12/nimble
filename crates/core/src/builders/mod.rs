pub mod docker;
pub mod go;

use std::path::Path;

use anyhow;

use crate::{
    builders::{docker::DockerBuilder, go::GoBuilder},
    config::BuilderType,
};

/// Represents a built Docker image
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// The full image reference (e.g., "myapp:latest" or "registry.example.com/myapp:v1.0.0")
    pub reference: String,
    /// Optional image digest (e.g., "sha256:abc123...")
    pub digest: Option<String>,
}

impl Image {
    /// Creates a new Image with just a reference
    pub fn new(reference: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            digest: None,
        }
    }

    /// Creates a new Image with a reference and digest
    pub fn with_digest(reference: impl Into<String>, digest: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            digest: Some(digest.into()),
        }
    }
}

/// Trait for building Docker images from source code.
#[async_trait::async_trait]
pub trait Builder: Send + Sync {
    /// Builds a Docker image from the source code at the given path.
    ///
    /// # Arguments
    ///
    /// * `build_path` - Path to the directory containing the source code to build
    /// * `image_name` - Name for the built image (e.g., "myapp" or "registry.com/myapp")
    /// * `image_tag` - Tag for the built image (e.g., "latest" or "v1.0.0")
    ///
    /// # Returns
    ///
    /// Returns the built `Image` with its reference and optional digest.
    async fn build(
        &self,
        build_path: &Path,
        image_name: &str,
        image_tag: &str,
    ) -> anyhow::Result<Image>;
}

pub fn select_builder(r#type: BuilderType) -> Box<dyn Builder> {
    match r#type {
        BuilderType::Dockerfile => Box::new(DockerBuilder::new()),
        BuilderType::Go => Box::new(GoBuilder::new()),
    }
}
