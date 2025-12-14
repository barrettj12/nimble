use std::{
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::{Context, Result};
use nimble_core::{builders::select_builder, config::NimbleConfig};
use serde::{Deserialize, Serialize};
use tar::Archive;
use tokio::{fs::create_dir_all, sync::mpsc::Receiver, task::spawn_blocking};
use tracing::{error, info};
use uuid::Uuid;

use crate::config::AgentConfig;

pub struct BuildJob {
    pub build_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
    Queued,
    Building,
    Success,
    Failed,
}

impl BuildStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildStatus::Queued => "queued",
            BuildStatus::Building => "building",
            BuildStatus::Success => "success",
            BuildStatus::Failed => "failed",
        }
    }
}

impl fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BuildStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(BuildStatus::Queued),
            "building" => Ok(BuildStatus::Building),
            "success" => Ok(BuildStatus::Success),
            "failed" => Ok(BuildStatus::Failed),
            _ => Err(format!("Unknown build status: {}", s)),
        }
    }
}

pub struct BuildWorker {
    config: Arc<AgentConfig>,
}

impl BuildWorker {
    pub fn new(config: Arc<AgentConfig>) -> Self {
        Self { config }
    }

    /// Runs the build worker, processing build jobs from the channel.
    pub async fn run(&self, mut build_rx: Receiver<BuildJob>) -> Result<()> {
        info!("Build worker started");

        while let Some(job) = build_rx.recv().await {
            let build_id = job.build_id;
            info!(build_id = %build_id, "Processing build job");

            if let Err(e) = self.process_build(job).await {
                error!(build_id = %build_id, error = %e, "Build failed");
                // Continue processing other jobs even if one fails
            }
        }

        info!("Build worker stopped (channel closed)");
        Ok(())
    }

    async fn process_build(&self, job: BuildJob) -> Result<()> {
        let source_archive_path = self.config.paths().source_archive(job.build_id);
        let build_dir = self.config.paths().build_dir(job.build_id);

        // Ensure that build directory exists
        create_dir_all(&build_dir)
            .await
            .with_context(|| format!("creating build directory {}", build_dir.display()))?;

        // Extract archive into build dir
        self.extract_archive(&source_archive_path, &build_dir)
            .await
            .with_context(|| format!("extracting archive {}", source_archive_path.display()))?;

        // Check for nimble.yaml file
        let nimble_yaml_path = build_dir.join("nimble.yaml");
        let has_nimble_yaml = tokio::fs::try_exists(&nimble_yaml_path)
            .await
            .with_context(|| format!("checking for nimble.yaml in {}", build_dir.display()))?;

        if !has_nimble_yaml {
            anyhow::bail!(
                "Cannot detect build type: nimble.yaml not found in build directory {}",
                build_dir.display()
            );

            // TODO: try auto-detecting the builder type
            // TODO: set build as failed in DB
        }

        let cfg = NimbleConfig::from_file(nimble_yaml_path)?;
        let builder = select_builder(cfg.builder_type);

        let image_name = format!("nimble-build-{}", job.build_id);
        let image_tag = "latest";

        let image = builder
            .build(&build_dir, &image_name, image_tag)
            .await
            .with_context(|| {
                format!(
                    "failed to build image for build_id {} using builder {:?}",
                    job.build_id, cfg.builder_type
                )
            })?;

        info!(
            build_id = %job.build_id,
            image_reference = %image.reference,
            image_digest = ?image.digest,
            "Build completed successfully"
        );
        // TODO: update image info in DB

        Ok(())
    }

    async fn extract_archive(&self, archive_path: &Path, extract_to: &Path) -> Result<()> {
        let archive_path = archive_path.to_owned();
        let extract_to = extract_to.to_owned();

        spawn_blocking(move || -> Result<()> {
            // Open archive file (blocking)
            let file = std::fs::File::open(&archive_path)
                .with_context(|| format!("opening archive {}", archive_path.display()))?;

            let gz = flate2::read::GzDecoder::new(file);
            let mut archive = Archive::new(gz);

            for entry in archive.entries()? {
                let mut entry = entry?;

                // Sanitize path
                let path = entry.path()?;
                let safe_path = sanitize_tar_path(&path, &extract_to)?;

                // Create parent dirs
                if let Some(parent) = safe_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Limit file size
                const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
                if entry.size() > MAX_FILE_SIZE {
                    anyhow::bail!(
                        "file {} exceeds max size ({} bytes)",
                        path.display(),
                        entry.size()
                    );
                }

                entry.unpack(&safe_path)?;
            }

            Ok(())
        })
        .await?
    }
}

fn sanitize_tar_path(entry_path: &Path, base: &Path) -> Result<PathBuf> {
    let mut out = base.to_path_buf();

    for component in entry_path.components() {
        match component {
            std::path::Component::Normal(c) => out.push(c),
            std::path::Component::CurDir => {}
            _ => {
                anyhow::bail!("invalid path component in archive entry: {:?}", entry_path);
            }
        }
    }

    Ok(out)
}
