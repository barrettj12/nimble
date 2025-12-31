use std::{fmt, str::FromStr};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{process::Command, sync::mpsc::Receiver};
use tracing::{error, info};
use uuid::Uuid;

use crate::db::Database;

pub struct DeployJob {
    pub deploy_id: Uuid,
    pub build_id: Uuid,
    pub image_reference: String,
    pub app_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeployStatus {
    Queued,
    Deploying,
    Running,
    Failed,
}

impl DeployStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeployStatus::Queued => "queued",
            DeployStatus::Deploying => "deploying",
            DeployStatus::Running => "running",
            DeployStatus::Failed => "failed",
        }
    }
}

impl fmt::Display for DeployStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DeployStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(DeployStatus::Queued),
            "deploying" => Ok(DeployStatus::Deploying),
            "running" => Ok(DeployStatus::Running),
            "failed" => Ok(DeployStatus::Failed),
            _ => Err(format!("Unknown deploy status: {s}")),
        }
    }
}

pub struct DeployWorker {
    db: Database,
}

impl DeployWorker {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn run(&self, mut deploy_rx: Receiver<DeployJob>) -> Result<()> {
        info!("Deploy worker started");

        while let Some(job) = deploy_rx.recv().await {
            let deploy_id = job.deploy_id;
            info!(deploy_id = %deploy_id, build_id = %job.build_id, "Processing deploy job");

            if let Err(e) = self.process_deploy(job).await {
                error!(deploy_id = %deploy_id, error = %e, "Deployment failed");
            }
        }

        info!("Deploy worker stopped (channel closed)");
        Ok(())
    }

    async fn process_deploy(&self, job: DeployJob) -> Result<()> {
        self.db
            .update_deployment_status(job.deploy_id, DeployStatus::Deploying)
            .await
            .context("Failed to update deploy status to deploying")?;

        let container_name = format!("nimble-deploy-{}", job.deploy_id);

        let output = Command::new("docker")
            .arg("run")
            .arg("-d")
            .arg("-p")
            .arg(format!("0:{}", job.app_port)) // publish app port to a random host port
            .arg("--name")
            .arg(&container_name)
            .arg(&job.image_reference)
            .output()
            .await
            .context("Failed to execute docker run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.db
                .update_deployment_status(job.deploy_id, DeployStatus::Failed)
                .await?;
            anyhow::bail!(
                "Docker run failed for deploy {}: {}\nStderr: {}",
                job.deploy_id,
                output.status,
                stderr
            );
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if container_id.is_empty() {
            self.db
                .update_deployment_status(job.deploy_id, DeployStatus::Failed)
                .await?;
            anyhow::bail!(
                "Docker run succeeded but no container ID returned for deploy {}",
                job.deploy_id
            );
        }

        let host_port = self.lookup_host_port(&container_name, job.app_port).await?;
        let address = host_port
            .as_ref()
            .map(|port| format!("http://127.0.0.1:{port}"));

        self.db
            .set_deployment_container(
                job.deploy_id,
                &container_id,
                &container_name,
                address.as_deref(),
            )
            .await
            .context("Failed to record container info")?;

        self.db
            .update_deployment_status(job.deploy_id, DeployStatus::Running)
            .await
            .context("Failed to update deploy status to running")?;

        info!(
            deploy_id = %job.deploy_id,
            build_id = %job.build_id,
            container_id = %container_id,
            container_name = %container_name,
            address = ?address,
            "Deployment started"
        );

        Ok(())
    }

    async fn lookup_host_port(
        &self,
        container_name: &str,
        app_port: u16,
    ) -> Result<Option<String>> {
        let output = Command::new("docker")
            .arg("port")
            .arg(container_name)
            .arg(format!("{app_port}/tcp"))
            .output()
            .await
            .context("Failed to query docker port mapping")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("docker port failed: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let host_port = stdout.lines().find_map(|line| {
            line.rsplit_once(':')
                .map(|(_, port)| port.trim().to_string())
        });

        Ok(host_port)
    }
}
