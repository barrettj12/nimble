use anyhow::{Context, Result};
use clap::Args;
use reqwest::StatusCode;

use crate::types::{DeploymentResponse, ErrorResponse};

#[derive(Args, Debug)]
pub struct DeploymentGetArgs {
    /// Deployment ID to fetch
    pub deployment_id: String,
}

pub async fn execute(agent_url: &str, args: &DeploymentGetArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{agent_url}/deployments/{}", args.deployment_id);
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to query deployment status")?;

    match response.status() {
        StatusCode::OK => {
            let deployment: DeploymentResponse = response
                .json()
                .await
                .context("Failed to parse deployment")?;
            print_deployment(&deployment);
            Ok(())
        }
        StatusCode::NOT_FOUND => {
            anyhow::bail!("Deployment not found: {}", args.deployment_id);
        }
        status => {
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: format!("HTTP {status}"),
            });
            anyhow::bail!("Failed to fetch deployment: {}", error.error);
        }
    }
}

pub fn print_deployment(deployment: &DeploymentResponse) {
    println!("Deployment ID: {}", deployment.id);
    println!("Build ID:      {}", deployment.build_id);
    println!("App:           {}", deployment.app);
    println!("Status:        {}", deployment.status);
    println!("Image:         {}", deployment.image);
    if let Some(address) = &deployment.address {
        println!("Address:       {}", address);
    }

    if let Some(container_id) = &deployment.container_id {
        println!("Container ID:  {}", container_id);
    }

    if let Some(container_name) = &deployment.container_name {
        println!("Container:     {}", container_name);
    }

    println!("Created At:    {}", deployment.created_at);
    println!("Updated At:    {}", deployment.updated_at);
}
