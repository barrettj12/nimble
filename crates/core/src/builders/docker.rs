use std::path::Path;

use async_trait::async_trait;
use tokio::process::Command;

use crate::builders::{Builder, Image};

pub struct DockerBuilder;

impl DockerBuilder {
    pub fn new() -> Self {
        DockerBuilder
    }
}

#[async_trait]
impl Builder for DockerBuilder {
    async fn build(
        &self,
        build_path: &Path,
        image_name: &str,
        image_tag: &str,
    ) -> anyhow::Result<Image> {
        // Check that Dockerfile exists
        let dockerfile_path = build_path.join("Dockerfile");
        if !dockerfile_path.exists() {
            anyhow::bail!(
                "Dockerfile not found in build directory: {}",
                build_path.display()
            );
        }

        // Build the full image reference
        let image_ref = format!("{image_name}:{image_tag}");

        // Run docker build
        let output = Command::new("docker")
            .arg("build")
            .arg("--tag")
            .arg(&image_ref)
            .arg("--file")
            .arg(&dockerfile_path)
            .arg(build_path)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute docker build: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Docker build failed: {}\nStderr: {}", output.status, stderr);
        }

        // Try to get the image digest
        let digest = get_image_digest(&image_ref).await.ok();

        Ok(Image {
            reference: image_ref,
            digest,
        })
    }
}

/// Gets the digest of a Docker image by inspecting it.
async fn get_image_digest(image_ref: &str) -> anyhow::Result<String> {
    let output = Command::new("docker")
        .arg("inspect")
        .arg("--format={{index .RepoDigests 0}}")
        .arg(image_ref)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to inspect image: {e}"))?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to inspect image: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let output_str = output_str.trim();

    // Extract digest from format like "image@sha256:abc123..."
    // If the output is empty or doesn't contain @, try getting the ID instead
    if output_str.is_empty() || !output_str.contains('@') {
        // Fallback: get the image ID
        let id_output = Command::new("docker")
            .arg("inspect")
            .arg("--format={{.Id}}")
            .arg(image_ref)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get image ID: {e}"))?;

        if id_output.status.success() {
            let id = String::from_utf8_lossy(&id_output.stdout)
                .trim()
                .to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
        anyhow::bail!("Could not determine image digest or ID");
    }

    // Extract the digest part (everything after @)
    if let Some(digest_part) = output_str.split('@').nth(1) {
        Ok(digest_part.to_string())
    } else {
        anyhow::bail!("Could not parse digest from inspect output: {output_str}")
    }
}
