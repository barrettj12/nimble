use anyhow::{Context, Result};
use clap::Args;

use crate::types::{BuildResponse, ErrorResponse};

#[derive(Args, Debug)]
pub struct BuildListArgs {
    /// Filter results by status (queued, building, success, failed)
    #[arg(long)]
    pub status: Option<String>,
    /// Limit number of results returned
    #[arg(long)]
    pub limit: Option<u64>,
}

pub async fn execute(agent_url: &str, args: &BuildListArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{agent_url}/builds");

    let mut query_params: Vec<(String, String)> = Vec::new();

    if let Some(status) = &args.status {
        query_params.push(("status".into(), status.clone()));
    }

    if let Some(limit) = args.limit {
        query_params.push(("limit".into(), limit.to_string()));
    }

    let request = if query_params.is_empty() {
        client.get(&url)
    } else {
        client.get(&url).query(&query_params)
    };

    let response = request
        .send()
        .await
        .context("Failed to send request to agent")?;

    let status = response.status();

    if status.is_success() {
        let builds: Vec<BuildResponse> =
            response.json().await.context("Failed to parse response")?;

        if builds.is_empty() {
            println!("No builds found.");
        } else {
            println!(
                "{:<40} {:<12} {:<20} {:<20}",
                "ID", "STATUS", "CREATED", "UPDATED"
            );
            println!("{}", "-".repeat(92));

            for build in builds {
                println!(
                    "{:<40} {:<12} {:<20} {:<20}",
                    build.id, build.status, build.created_at, build.updated_at
                );
            }
        }
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            error: format!("HTTP {status}"),
        });
        anyhow::bail!("Failed to list builds: {}", error.error);
    }

    Ok(())
}
