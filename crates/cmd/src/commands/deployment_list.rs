use anyhow::{Context, Result};
use clap::Args;
use reqwest::StatusCode;

use crate::types::{DeploymentResponse, ErrorResponse};

#[derive(Args, Debug)]
pub struct DeploymentListArgs {
    /// Filter by build ID
    #[arg(long)]
    pub build_id: Option<String>,
}

pub async fn execute(agent_url: &str, args: &DeploymentListArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{agent_url}/deployments");
    if let Some(build_id) = &args.build_id {
        url.push_str(&format!("?build_id={build_id}"));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to query deployments")?;

    match response.status() {
        StatusCode::OK => {
            let deployments: Vec<DeploymentResponse> = response
                .json()
                .await
                .context("Failed to parse deployments")?;

            if deployments.is_empty() {
                println!("No deployments found.");
                return Ok(());
            }

            for deployment in deployments {
                println!(
                    "{}  {}  {}",
                    deployment.id, deployment.status, deployment.image
                );
                println!("  app:   {}", deployment.app);
                println!("  build: {}", deployment.build_id);
                if let Some(address) = &deployment.address {
                    println!("  address: {}", address);
                }
                if let Some(container_name) = &deployment.container_name {
                    println!("  container: {}", container_name);
                }
                println!("  created: {}", deployment.created_at);
                println!();
            }

            Ok(())
        }
        status => {
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: format!("HTTP {status}"),
            });
            anyhow::bail!("Failed to list deployments: {}", error.error);
        }
    }
}
