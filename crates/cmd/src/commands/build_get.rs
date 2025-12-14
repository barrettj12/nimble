use anyhow::{Context, Result};
use clap::Args;
use reqwest::StatusCode;

use crate::types::{BuildResponse, ErrorResponse};

#[derive(Args, Debug)]
pub struct BuildGetArgs {
    /// Build ID to fetch
    pub id: String,
}

pub async fn execute(agent_url: &str, args: &BuildGetArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/builds/{}", agent_url, args.id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to agent")?;

    let status = response.status();

    if status.is_success() {
        let build: BuildResponse = response.json().await.context("Failed to parse response")?;

        println!("Build Details:");
        println!("  ID:       {}", build.id);
        println!("  Status:   {}", build.status);
        println!("  Created:  {}", build.created_at);
        println!("  Updated:  {}", build.updated_at);
    } else if status == StatusCode::NOT_FOUND {
        anyhow::bail!("Build not found: {}", args.id);
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            error: format!("HTTP {status}"),
        });
        anyhow::bail!("Failed to get build: {}", error.error);
    }

    Ok(())
}
