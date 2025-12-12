use serde::Deserialize;
use std::{env, fs, path::Path};
mod builders;
mod types;
use builders::{Builder, GoBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Determine project path
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        // TODO: try to get cwd
        return Err("Must provide project path".into());
    }
    let project_path = Path::new(&args[1]);
    // TODO: stat project path

    // Read config
    let project_cfg_raw = fs::read_to_string(project_path.join("nimble.yaml"))?;
    let project_cfg: ProjectConfig = serde_yaml::from_str(&project_cfg_raw)?;

    let _builder = get_builder(&project_cfg.builder);
    // let image = builder.build(project_path);

    // let deploy;
    // deployer.deploy(image);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    builder: String,
    deploy: String,
}

fn get_builder(r#type: &str) -> Result<Box<dyn Builder>, String> {
    match r#type {
        "go" => Ok(Box::new(GoBuilder::new())),
        _ => Err(format!("Unknown builder type: {}", r#type)),
    }
}


