//! React Native bundle building functionality for CLI
//!
//! This module provides functionality to build React Native JavaScript bundles
//! from source code, including platform-specific configurations and optimization.

use rodepush_core::{Bundle, BundleBuilder, BundleError, Platform, Result, SemanticVersion};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tracing::{debug, error, info, warn};

/// Configuration for React Native bundle building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// React Native project directory
    pub project_dir: PathBuf,
    /// Entry point file (e.g., "index.js")
    pub entry_file: String,
    /// Target platform
    pub platform: Platform,
    /// Output directory for built bundles
    pub output_dir: PathBuf,
    /// Whether to enable minification
    pub minify: bool,
    /// Whether to enable source maps
    pub source_maps: bool,
    /// Additional Metro bundler options
    pub metro_options: Vec<String>,
    /// Environment variables for the build process
    pub env_vars: std::collections::HashMap<String, String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            entry_file: "index.js".to_string(),
            platform: Platform::Both,
            output_dir: PathBuf::from("./build"),
            minify: true,
            source_maps: false,
            metro_options: Vec::new(),
            env_vars: std::collections::HashMap::new(),
        }
    }
}

/// Result of a React Native build operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    /// The built bundle
    pub bundle: Bundle,
    /// Path to the generated bundle file
    pub bundle_path: PathBuf,
    /// Path to the source map file (if generated)
    pub source_map_path: Option<PathBuf>,
    /// Build duration in milliseconds
    pub build_duration_ms: u64,
    /// Bundle size in bytes
    pub bundle_size_bytes: u64,
    /// Whether the build was successful
    pub success: bool,
    /// Error message if build failed
    pub error_message: Option<String>,
}

/// Entry point for React Native applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    /// Entry file path
    pub file: PathBuf,
    /// Entry file content hash
    pub content_hash: String,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Bundle source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSource {
    /// Source directory
    pub source_dir: PathBuf,
    /// Entry point
    pub entry_point: EntryPoint,
    /// Platform-specific source files
    pub platform_files: std::collections::HashMap<Platform, Vec<PathBuf>>,
}

/// React Native bundle builder
pub struct ReactNativeBuilder {
    config: BuildConfig,
}

impl ReactNativeBuilder {
    /// Create a new React Native builder
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    /// Build a React Native bundle
    pub async fn build(&self) -> Result<BuildResult> {
        let start_time = std::time::Instant::now();

        info!(
            "Starting React Native bundle build for platform: {}",
            self.config.platform
        );
        debug!("Project directory: {:?}", self.config.project_dir);
        debug!("Entry file: {}", self.config.entry_file);
        debug!("Output directory: {:?}", self.config.output_dir);

        // Validate project structure
        self.validate_project_structure()?;

        // Create output directory
        std::fs::create_dir_all(&self.config.output_dir).map_err(|e| {
            BundleError::build_failed(format!("Failed to create output directory: {}", e))
        })?;

        // Build for target platforms
        let mut bundles = Vec::new();
        let platforms = self.get_target_platforms();

        for platform in platforms {
            info!("Building for platform: {}", platform);
            let bundle = self.build_for_platform(platform).await?;
            bundles.push(bundle);
        }

        // Combine bundles if building for multiple platforms
        let final_bundle = if bundles.len() == 1 {
            bundles.into_iter().next().unwrap()
        } else {
            self.combine_platform_bundles(bundles).await?
        };

        let build_duration = start_time.elapsed();
        let bundle_size = final_bundle.size();

        info!("Build completed successfully in {:?}", build_duration);
        info!("Bundle size: {} bytes", bundle_size);

        Ok(BuildResult {
            bundle: final_bundle,
            bundle_path: self.config.output_dir.join(format!(
                "bundle.{}",
                self.config.platform.bundle_extension()
            )),
            source_map_path: if self.config.source_maps {
                Some(self.config.output_dir.join("bundle.map"))
            } else {
                None
            },
            build_duration_ms: build_duration.as_millis() as u64,
            bundle_size_bytes: bundle_size,
            success: true,
            error_message: None,
        })
    }

    /// Validate React Native project structure
    fn validate_project_structure(&self) -> Result<()> {
        let project_dir = &self.config.project_dir;

        // Check if project directory exists
        if !project_dir.exists() {
            return Err(BundleError::build_failed(format!(
                "Project directory does not exist: {:?}",
                project_dir
            ))
            .into());
        }

        // Check for package.json
        let package_json = project_dir.join("package.json");
        if !package_json.exists() {
            return Err(BundleError::build_failed(
                "package.json not found in project directory".to_string(),
            )
            .into());
        }

        // Check for entry file
        let entry_file = project_dir.join(&self.config.entry_file);
        if !entry_file.exists() {
            return Err(BundleError::build_failed(format!(
                "Entry file not found: {:?}",
                entry_file
            ))
            .into());
        }

        // Check for React Native dependencies
        self.check_react_native_dependencies()?;

        Ok(())
    }

    /// Check if React Native dependencies are installed
    fn check_react_native_dependencies(&self) -> Result<()> {
        let package_json = self.config.project_dir.join("package.json");
        let package_content = std::fs::read_to_string(&package_json).map_err(|e| {
            BundleError::build_failed(format!("Failed to read package.json: {}", e))
        })?;

        let package: serde_json::Value = serde_json::from_str(&package_content).map_err(|e| {
            BundleError::build_failed(format!("Failed to parse package.json: {}", e))
        })?;

        let dependencies = package
            .get("dependencies")
            .and_then(|d| d.as_object())
            .ok_or_else(|| {
                BundleError::build_failed("No dependencies found in package.json".to_string())
            })?;

        // Check for React Native
        if !dependencies.contains_key("react-native") {
            return Err(BundleError::build_failed(
                "react-native dependency not found in package.json".to_string(),
            )
            .into());
        }

        // Check for Metro bundler
        if !dependencies.contains_key("@react-native/metro-config")
            && !dependencies.contains_key("metro-config")
        {
            warn!("Metro bundler not found in dependencies, using default configuration");
        }

        Ok(())
    }

    /// Get target platforms for building
    fn get_target_platforms(&self) -> Vec<Platform> {
        match self.config.platform {
            Platform::Both => vec![Platform::Ios, Platform::Android],
            platform => vec![platform],
        }
    }

    /// Build bundle for a specific platform
    async fn build_for_platform(&self, platform: Platform) -> Result<Bundle> {
        let bundle_name = format!("bundle.{}", platform.bundle_extension());
        let bundle_path = self.config.output_dir.join(&bundle_name);

        // Use React Native CLI to build the bundle
        let bundle_data = self.run_react_native_build(platform).await?;

        // Write bundle to file
        std::fs::write(&bundle_path, &bundle_data).map_err(|e| {
            BundleError::build_failed(format!("Failed to write bundle file: {}", e))
        })?;

        // Create bundle metadata
        let version = self.extract_version_from_package_json()?;
        let mut builder = BundleBuilder::new(version, platform, self.config.entry_file.clone());

        // Add bundle data as a single chunk
        builder.add_chunk_from_data(&bundle_data, "main".to_string())?;

        // Build the bundle
        let bundle = builder.build()?;

        Ok(bundle)
    }

    /// Run React Native CLI build command
    async fn run_react_native_build(&self, platform: Platform) -> Result<Vec<u8>> {
        let mut command = Command::new("npx");
        command.arg("react-native");
        command.arg("bundle");

        // Set platform
        command.arg("--platform");
        command.arg(match platform {
            Platform::Ios => "ios",
            Platform::Android => "android",
            Platform::Both => "both",
        });

        // Set entry file
        command.arg("--entry-file");
        command.arg(&self.config.entry_file);

        // Set output
        let bundle_name = format!("bundle.{}", platform.bundle_extension());
        let bundle_path = self.config.output_dir.join(&bundle_name);
        command.arg("--bundle-output");
        command.arg(bundle_path.to_str().unwrap());

        // Set source map output if enabled
        if self.config.source_maps {
            let source_map_path = self.config.output_dir.join("bundle.map");
            command.arg("--sourcemap-output");
            command.arg(source_map_path.to_str().unwrap());
        }

        // Set minification
        if self.config.minify {
            command.arg("--minify");
        } else {
            command.arg("--no-minify");
        }

        // Add additional Metro options
        for option in &self.config.metro_options {
            command.arg(option);
        }

        // Set working directory
        command.current_dir(&self.config.project_dir);

        // Set environment variables
        for (key, value) in &self.config.env_vars {
            command.env(key, value);
        }

        // Set default environment variables
        command.env(
            "NODE_ENV",
            if self.config.minify {
                "production"
            } else {
                "development"
            },
        );

        debug!("Running command: {:?}", command);

        // Execute the command
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                BundleError::build_failed(format!("Failed to execute React Native CLI: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("React Native build failed: {}", stderr);
            return Err(BundleError::build_failed(format!(
                "React Native build failed: {}",
                stderr
            ))
            .into());
        }

        // Read the generated bundle file
        let bundle_name = format!("bundle.{}", platform.bundle_extension());
        let bundle_path = self.config.output_dir.join(&bundle_name);

        if !bundle_path.exists() {
            return Err(
                BundleError::build_failed("Bundle file was not generated".to_string()).into(),
            );
        }

        let bundle_data = std::fs::read(&bundle_path).map_err(|e| {
            BundleError::build_failed(format!("Failed to read generated bundle: {}", e))
        })?;

        Ok(bundle_data)
    }

    /// Extract version from package.json
    fn extract_version_from_package_json(&self) -> Result<SemanticVersion> {
        let package_json = self.config.project_dir.join("package.json");
        let package_content = std::fs::read_to_string(&package_json).map_err(|e| {
            BundleError::build_failed(format!("Failed to read package.json: {}", e))
        })?;

        let package: serde_json::Value = serde_json::from_str(&package_content).map_err(|e| {
            BundleError::build_failed(format!("Failed to parse package.json: {}", e))
        })?;

        let version_str = package
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BundleError::build_failed("Version not found in package.json".to_string())
            })?;

        SemanticVersion::parse(version_str)
    }

    /// Combine bundles from multiple platforms
    async fn combine_platform_bundles(&self, bundles: Vec<Bundle>) -> Result<Bundle> {
        // For now, we'll just return the first bundle
        // In a more sophisticated implementation, we might want to create a universal bundle
        if bundles.is_empty() {
            return Err(BundleError::build_failed("No bundles to combine".to_string()).into());
        }

        Ok(bundles.into_iter().next().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert_eq!(config.entry_file, "index.js");
        assert_eq!(config.platform, Platform::Both);
        assert!(config.minify);
        assert!(!config.source_maps);
    }

    #[test]
    fn test_entry_point_creation() {
        let entry = EntryPoint {
            file: PathBuf::from("index.js"),
            content_hash: "abc123".to_string(),
            dependencies: vec!["react".to_string(), "react-native".to_string()],
        };

        assert_eq!(entry.file, PathBuf::from("index.js"));
        assert_eq!(entry.content_hash, "abc123");
        assert_eq!(entry.dependencies.len(), 2);
    }

    #[tokio::test]
    async fn test_react_native_builder_creation() {
        let config = BuildConfig::default();
        let builder = ReactNativeBuilder::new(config);

        // This should not panic
        assert!(true);
    }
}
