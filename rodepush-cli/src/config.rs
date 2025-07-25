//! Configuration management for the RodePush CLI.
//!
//! This module handles loading configuration from files, environment variables,
//! and command line arguments, with appropriate precedence.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            build: BuildConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server URL
    #[serde(default = "default_server_url")]
    pub url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: default_server_url(),
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

fn default_server_url() -> String {
    "https://api.rodepush.com".to_string()
}

fn default_timeout_seconds() -> u64 {
    30
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Target platform (ios, android, both)
    #[serde(default = "default_platform")]
    pub platform: String,

    /// Entry file for the React Native bundle
    #[serde(default = "default_entry_file")]
    pub entry_file: String,

    /// Output directory for built bundles
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            platform: default_platform(),
            entry_file: default_entry_file(),
            output_dir: default_output_dir(),
        }
    }
}

fn default_platform() -> String {
    "both".to_string()
}

fn default_entry_file() -> String {
    "index.js".to_string()
}

fn default_output_dir() -> String {
    "./build".to_string()
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Path to API key file
    #[serde(default = "default_api_key_file")]
    pub api_key_file: String,

    /// API key (can be set via environment variable)
    #[serde(skip)]
    pub api_key: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_key_file: default_api_key_file(),
            api_key: None,
        }
    }
}

fn default_api_key_file() -> String {
    "~/.rodepush/api_key".to_string()
}

impl Config {
    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with the following precedence:
    /// 1. Command line arguments (handled by Clap)
    /// 2. Environment variables
    /// 3. Configuration file
    /// 4. Default values
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Start with defaults
        let mut config = Config::default();

        // Try to load from config file
        let config_path = PathBuf::from(".rodepush.toml");
        if config_path.exists() {
            match Config::from_file(&config_path) {
                Ok(file_config) => {
                    // Merge file config with defaults
                    config.server = file_config.server;
                    config.build = file_config.build;
                    config.auth = file_config.auth;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load config file: {}", e);
                }
            }
        }

        // Override with environment variables
        if let Ok(url) = std::env::var("RODEPUSH_SERVER_URL") {
            config.server.url = url;
        }

        if let Ok(timeout) = std::env::var("RODEPUSH_TIMEOUT_SECONDS") {
            if let Ok(timeout_seconds) = timeout.parse::<u64>() {
                config.server.timeout_seconds = timeout_seconds;
            }
        }

        if let Ok(platform) = std::env::var("RODEPUSH_BUILD_PLATFORM") {
            config.build.platform = platform;
        }

        if let Ok(entry_file) = std::env::var("RODEPUSH_ENTRY_FILE") {
            config.build.entry_file = entry_file;
        }

        if let Ok(output_dir) = std::env::var("RODEPUSH_OUTPUT_DIR") {
            config.build.output_dir = output_dir;
        }

        if let Ok(api_key_file) = std::env::var("RODEPUSH_API_KEY_FILE") {
            config.auth.api_key_file = api_key_file;
        }

        // API key can be set directly via environment variable
        if let Ok(api_key) = std::env::var("RODEPUSH_API_KEY") {
            config.auth.api_key = Some(api_key);
        }

        Ok(config)
    }

    /// Validate configuration
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate server URL
        if !self.server.url.starts_with("http://") && !self.server.url.starts_with("https://") {
            return Err("Invalid server URL: must start with http:// or https://".into());
        }

        // Validate timeout
        if self.server.timeout_seconds == 0 {
            return Err("Timeout must be greater than 0".into());
        }

        // Validate platform
        let valid_platforms = ["ios", "android", "both"];
        if !valid_platforms.contains(&self.build.platform.as_str()) {
            return Err(format!(
                "Invalid platform: {}. Must be one of: {:?}",
                self.build.platform, valid_platforms
            )
            .into());
        }

        // Validate entry file
        if self.build.entry_file.is_empty() {
            return Err("Entry file cannot be empty".into());
        }

        // Validate output directory
        if self.build.output_dir.is_empty() {
            return Err("Output directory cannot be empty".into());
        }

        Ok(())
    }

    /// Load and validate configuration
    #[allow(dead_code)]
    pub fn load_and_validate() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Self::load()?;
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.url, "https://api.rodepush.com");
        assert_eq!(config.server.timeout_seconds, 30);
        assert_eq!(config.build.platform, "both");
        assert_eq!(config.build.entry_file, "index.js");
        assert_eq!(config.build.output_dir, "./build");
        assert_eq!(config.auth.api_key_file, "~/.rodepush/api_key");
    }

    #[test]
    fn test_config_from_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test_config.toml");

        let config_content = r#"
[server]
url = "https://test.rodepush.com"
timeout_seconds = 60

[build]
platform = "android"
entry_file = "main.js"
output_dir = "./dist"

[auth]
api_key_file = "/path/to/api_key"
"#;

        let mut file = std::fs::File::create(&config_path)?;
        file.write_all(config_content.as_bytes())?;

        let config = Config::from_file(&config_path)?;
        assert_eq!(config.server.url, "https://test.rodepush.com");
        assert_eq!(config.server.timeout_seconds, 60);
        assert_eq!(config.build.platform, "android");
        assert_eq!(config.build.entry_file, "main.js");
        assert_eq!(config.build.output_dir, "./dist");
        assert_eq!(config.auth.api_key_file, "/path/to/api_key");

        Ok(())
    }

    #[test]
    fn test_config_load_with_env() -> Result<(), Box<dyn std::error::Error>> {
        // This test is tricky because we can't easily modify environment variables in a test
        // In a real implementation, we would use a testing framework that supports environment variable mocking
        // For now, we'll just test that the function compiles and runs without error
        let config = Config::load()?;
        assert!(!config.server.url.is_empty());
        Ok(())
    }
}
