use std::{
    fs::File,
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Args;
use flate2::{Compression, write::GzEncoder};
use reqwest::StatusCode;
use tar::Builder;
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::types::{BuildResponse, CreateBuildResponse, ErrorResponse};

const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Args, Debug)]
pub struct DeployArgs {
    /// Directory containing the source to deploy
    pub directory: PathBuf,
    /// Block until the build finishes
    #[arg(long)]
    pub wait: bool,
}

pub async fn execute(agent_url: &str, args: &DeployArgs) -> Result<()> {
    let archive =
        create_tarball(&args.directory).with_context(|| "Failed to create deployment archive")?;

    let client = reqwest::Client::new();
    let url = format!("{agent_url}/builds");

    let response = client
        .post(&url)
        .header("Content-Type", "application/gzip")
        .body(archive)
        .send()
        .await
        .context("Failed to send request to agent")?;

    let status = response.status();

    if status.is_success() {
        let build: CreateBuildResponse =
            response.json().await.context("Failed to parse response")?;

        println!("Build created successfully!");
        println!("Build ID: {}", build.build_id);
        println!("Status: {}", build.status);

        if args.wait {
            wait_for_completion(agent_url, &build.build_id).await?;
        }
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            error: format!("HTTP {status}"),
        });
        anyhow::bail!("Failed to create build: {}", error.error);
    }

    Ok(())
}

async fn wait_for_completion(agent_url: &str, build_id: &str) -> Result<()> {
    println!("Waiting for build {build_id} to finish...");
    let client = reqwest::Client::new();
    let mut last_reported_status: Option<String> = None;

    loop {
        let url = format!("{agent_url}/builds/{build_id}");
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to poll build status")?;

        match response.status() {
            StatusCode::OK => {
                let build: BuildResponse = response
                    .json()
                    .await
                    .context("Failed to parse build status")?;
                if last_reported_status.as_deref() != Some(build.status.as_str()) {
                    println!("Status: {}", build.status);
                    last_reported_status = Some(build.status.clone());
                }
                match build.status.as_str() {
                    "success" | "succeeded" => {
                        println!("Build finished successfully.");
                        return Ok(());
                    }
                    "failed" | "errored" => {
                        anyhow::bail!("Build failed: {}", build.id);
                    }
                    _ => {
                        sleep(POLL_INTERVAL).await;
                    }
                }
            }
            StatusCode::NOT_FOUND => {
                anyhow::bail!("Build not found: {build_id}");
            }
            status => {
                let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                    error: format!("HTTP {status}"),
                });
                anyhow::bail!("Failed to fetch build status: {}", error.error);
            }
        }
    }
}

fn create_tarball(dir: &Path) -> Result<Vec<u8>> {
    let directory = dir
        .canonicalize()
        .with_context(|| format!("Directory does not exist: {}", dir.display()))?;

    let cursor = Cursor::new(Vec::new());
    let encoder = GzEncoder::new(cursor, Compression::default());
    let mut builder = Builder::new(encoder);

    for entry in WalkDir::new(&directory) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(&directory).unwrap();

        if relative.as_os_str().is_empty() {
            continue;
        }

        let name = relative.to_string_lossy().replace('\\', "/");

        if entry.file_type().is_dir() {
            builder
                .append_dir(Path::new(&name), path)
                .with_context(|| format!("Failed to add directory: {}", path.display()))?;
            continue;
        }

        if entry.file_type().is_file() {
            let mut file = File::open(path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;
            builder
                .append_file(Path::new(&name), &mut file)
                .with_context(|| format!("Failed to add file to archive: {}", path.display()))?;
        }
    }

    builder.finish()?;
    let encoder = builder.into_inner()?;
    let cursor = encoder.finish()?;
    Ok(cursor.into_inner())
}
