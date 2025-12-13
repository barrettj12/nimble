use std::path::PathBuf;

use uuid::Uuid;

/// RunMode tells the agent whether it is running in a development or production environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Dev,
    Prod,
}

impl RunMode {
    /// Determines the run mode from the `NIMBLE_DEV_MODE` environment variable.
    ///
    /// Dev mode is selected if the variable is set to a "truthy" value:
    /// `"1"`, `"true"`, `"yes"`, `"on"` (case-insensitive).
    /// Any other value or if the variable is unset defaults to production mode.
    pub fn from_env() -> Self {
        match std::env::var("NIMBLE_DEV_MODE") {
            Ok(val) => {
                let val = val.to_lowercase();
                match val.as_str() {
                    "1" | "true" | "yes" | "on" => RunMode::Dev,
                    _ => RunMode::Prod,
                }
            }
            Err(_) => RunMode::Prod,
        }
    }
}

/// AgentConfig holds the config for the agent.
#[derive(Clone)]
pub struct AgentConfig {
    // run_mode determines if the agent is running in dev or prod mode.
    run_mode: RunMode,
    // data_dir determines where the agent stores its data.
    data_dir: Option<PathBuf>,
}

impl AgentConfig {
    pub fn new() -> Self {
        Self {
            run_mode: RunMode::from_env(),
            data_dir: None,
        }
    }

    /// Returns the data directory for the agent.
    ///
    /// Resolution order:
    /// 1. If `data_dir` is explicitly set in config, return that.
    /// 2. Otherwise, use defaults based on `run_mode`:
    ///    - Dev: `./data`
    ///    - Prod: `/var/lib/nimble`
    pub fn get_data_dir(&self) -> PathBuf {
        if let Some(ref dir) = self.data_dir {
            dir.clone()
        } else {
            match self.run_mode {
                RunMode::Dev => PathBuf::from("./data"),
                RunMode::Prod => PathBuf::from("/var/lib/nimble"),
            }
        }
    }

    // Return the paths helper.
    pub fn paths(&self) -> Paths {
        return Paths {
            base_dir: self.get_data_dir(),
        };
    }
}

// Paths contains convenience methods to generate paths to certain artifacts.
struct Paths {
    base_dir: PathBuf,
}

impl Paths {
    // Returns the path to store a zipped source archive.
    pub fn source_archive(&self, build_id: Uuid) -> PathBuf {
        self.base_dir
            .join("artifacts")
            .join("source")
            .join(format!("{}.tar.gz", build_id))
    }
}
