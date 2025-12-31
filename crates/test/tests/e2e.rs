use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command as StdCommand, Stdio},
    sync::OnceLock,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use serde::Deserialize;
use tempfile::{TempDir, tempdir};
use tokio::{process::Command as TokioCommand, time::sleep};

const AGENT_PORT: u16 = 7080;
const AGENT_URL: &str = "http://127.0.0.1:7080";

#[derive(Deserialize, Debug)]
struct BuildResponse {
    status: String,
}

#[derive(Deserialize, Debug)]
struct DeploymentResponse {
    id: String,
    status: String,
    image: String,
    build_id: String,
    container_name: Option<String>,
    address: Option<String>,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deploys_example_app() -> Result<()> {
    println!("Starting nimble e2e test");

    ensure_port_available(AGENT_PORT)?;
    ensure_docker_available()?;

    let data_dir = tempdir().context("create temp data dir")?;
    println!("Using temp data dir at {}", data_dir.path().display());

    let agent = spawn_agent(&data_dir)?;
    let mut agent_guard = ChildGuard::new(agent);

    println!("Waiting for agent health at {AGENT_URL}");
    wait_for_health().await?;

    println!("Agent is healthy, deploying sample app");
    let cli_output = run_cli_deploy().await?;
    let build_id = extract_build_id(&cli_output);

    verify_latest_build().await?;

    if let Some(build_id) = &build_id {
        println!("Verifying deployed app responds (build {build_id})");
        verify_deployed_app(build_id).await?;
    }

    if let Some(build_id) = build_id {
        println!("Cleaning up built image for build {build_id}");
        cleanup_image(&build_id).await.ok();
    }

    agent_guard.kill().ok();
    println!("E2E test complete");
    Ok(())
}

fn ensure_port_available(port: u16) -> Result<()> {
    if TcpListener::bind(("127.0.0.1", port)).is_ok() {
        Ok(())
    } else {
        bail!("port {port} is not available for the agent")
    }
}

fn ensure_docker_available() -> Result<()> {
    let status = StdCommand::new("docker")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("docker is installed but returned exit code {status}"),
        Err(err) => bail!("docker is not available: {err}"),
    }
}

fn spawn_agent(data_dir: &TempDir) -> Result<Child> {
    ensure_binaries_built()?;

    let binary = binary_path("nimbled")?;
    println!("Launching agent binary at {}", binary.display(),);
    StdCommand::new(binary)
        .env("NIMBLE_DEV_MODE", "1")
        .env("NIMBLE_DATA_DIR", data_dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn nimbled")
}

async fn wait_for_health() -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{AGENT_URL}/builds");

    for _ in 0..30 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status() == StatusCode::OK => return Ok(()),
            Ok(_) => {}
            Err(_) => {}
        }
        sleep(Duration::from_millis(500)).await;
    }

    bail!("agent did not become healthy at {url}")
}

async fn run_cli_deploy() -> Result<String> {
    ensure_binaries_built()?;

    let binary = binary_path("nimble")?;
    let project_dir = workspace_path(["examples", "go-hello"])?;

    println!(
        "Running CLI deploy from {} against {AGENT_URL}",
        project_dir.display()
    );
    let output = TokioCommand::new(binary)
        .arg("deploy")
        .arg(project_dir)
        .arg("--wait")
        .output()
        .await
        .context("run nimble deploy")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "nimble deploy failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            stdout,
            stderr
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("Build finished successfully") {
        bail!("deployment did not finish successfully; stdout:\n{stdout}");
    }

    Ok(stdout.into_owned())
}

async fn verify_latest_build() -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{AGENT_URL}/builds");
    println!("Fetching builds from {url}");
    let resp = client
        .get(&url)
        .send()
        .await
        .context("request build list")?
        .error_for_status()
        .context("build list returned error")?;

    let builds: Vec<BuildResponse> = resp.json().await.context("decode build list")?;

    if builds.is_empty() {
        bail!("no builds returned from agent after deployment");
    }

    if builds.iter().any(|b| b.status == "success") {
        Ok(())
    } else {
        bail!("no successful build found; statuses: {:?}", builds)
    }
}

async fn verify_deployed_app(build_id: &str) -> Result<()> {
    let deployment = fetch_latest_deployment(build_id).await?;
    let address = deployment
        .address
        .ok_or_else(|| anyhow::anyhow!("deployment missing address"))?;

    println!("Pinging app at {} for build {}", address, build_id);

    let client = reqwest::Client::new();
    let resp = client
        .get(&address)
        .send()
        .await
        .context("request deployed app")?
        .error_for_status()
        .context("deployed app returned error")?;

    let body = resp.text().await.context("read deployed app body")?;
    if !body.contains("Hello, World!") {
        bail!("unexpected app response: {body}");
    }

    println!("App responded successfully: {}", body.trim());
    Ok(())
}

async fn fetch_latest_deployment(build_id: &str) -> Result<DeploymentResponse> {
    let client = reqwest::Client::new();
    let url = format!("{AGENT_URL}/deployments?build_id={build_id}");
    let resp = client
        .get(&url)
        .send()
        .await
        .context("request deployment list")?
        .error_for_status()
        .context("deployment list returned error")?;

    let mut deployments: Vec<DeploymentResponse> =
        resp.json().await.context("decode deployment list")?;

    deployments
        .drain(..)
        .next()
        .ok_or_else(|| anyhow::anyhow!("no deployment found for build {build_id}"))
}

async fn cleanup_image(build_id: &str) -> Result<()> {
    let image_ref = format!("nimble-build-{build_id}:latest");
    let status = TokioCommand::new("docker")
        .arg("rmi")
        .arg("-f")
        .arg(&image_ref)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("remove build image")?;

    if status.success() {
        Ok(())
    } else {
        bail!("failed to remove build image {image_ref}")
    }
}

fn extract_build_id(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        line.strip_prefix("Build ID: ")
            .map(|id| id.trim().to_string())
    })
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .context("determine workspace root")
        .map(|p| p.to_path_buf())
}

fn workspace_path(parts: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<PathBuf> {
    let mut path = workspace_root()?;

    for part in parts {
        path.push(part.as_ref());
    }
    Ok(path)
}

fn target_dir() -> Result<PathBuf> {
    workspace_path(["target", "debug"])
}

fn binary_path(name: &str) -> Result<PathBuf> {
    let mut path = target_dir()?;
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    path.push(file);
    Ok(path)
}

fn ensure_binaries_built() -> Result<()> {
    static BUILT: OnceLock<Result<()>> = OnceLock::new();
    let res: &Result<()> = BUILT.get_or_init(|| {
        let workspace_root = workspace_root()?;
        let status = StdCommand::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("nimble-agent")
            .arg("-p")
            .arg("nimble")
            .current_dir(&workspace_root)
            .status()
            .context("build binaries for e2e test")?;

        if status.success() {
            Ok(())
        } else {
            bail!("cargo build -p nimble-agent -p nimble failed with {status}");
        }
    });
    res.as_ref()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
