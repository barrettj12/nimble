use std::{fs, path::Path, str::FromStr};

use serde::{Deserialize, Serialize};

/// Builder type for building the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuilderType {
    /// Use a Dockerfile to build the application
    Dockerfile,
    /// Use the Go builder
    Go,
}

impl FromStr for BuilderType {
    type Err = ConfigError;

    /// Parses a BuilderType from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - String representation of the builder type (case-insensitive)
    ///
    /// # Errors
    ///
    /// Returns an error if the string doesn't match any known builder type.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dockerfile" => Ok(BuilderType::Dockerfile),
            "go" => Ok(BuilderType::Go),
            _ => Err(ConfigError::InvalidBuilder(s.to_string())),
        }
    }
}

/// NimbleConfig represents the configuration from a nimble.yaml file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NimbleConfig {
    /// The builder type to use
    pub builder_type: BuilderType,
    /// Logical application name for grouping deployments
    pub app: String,
    /// Application port exposed by the built image/container. Defaults to 8080.
    #[serde(default = "default_app_port")]
    pub port: u16,
}

const fn default_app_port() -> u16 {
    8080
}

impl NimbleConfig {
    /// Loads a NimbleConfig from a nimble.yaml file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the nimble.yaml file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        Self::from_str(&contents)
    }
}

impl FromStr for NimbleConfig {
    type Err = ConfigError;

    fn from_str(yaml: &str) -> Result<Self, Self::Err> {
        let raw: serde_yaml::Value =
            serde_yaml::from_str(yaml).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Extract builder
        let builder_str = raw
            .get("builder")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigError::MissingField("builder".to_string()))?;

        let builder_type = BuilderType::from_str(builder_str)?;

        // Optional port with default
        let port = raw
            .get("port")
            .and_then(|v| v.as_u64())
            .map(|v| v as u16)
            .unwrap_or_else(default_app_port);

        // Application name
        let app = raw
            .get("app")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigError::MissingField("app".to_string()))?
            .to_string();

        Ok(NimbleConfig {
            builder_type,
            app,
            port,
        })
    }
}

/// Errors that can occur when loading or parsing a NimbleConfig.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// I/O error reading the file
    IoError(String),
    /// Error parsing the YAML
    ParseError(String),
    /// Required field is missing
    MissingField(String),
    /// Invalid builder type
    InvalidBuilder(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "I/O error: {msg}"),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            ConfigError::MissingField(field) => write!(f, "Missing required field: {field}"),
            ConfigError::InvalidBuilder(builder) => {
                write!(
                    f,
                    "Invalid builder type: {builder}. Valid options: dockerfile, go"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}
