use clap::Parser;
use rodepush_core::{
    init_logging, LogContext, LogConfig, LogFormat,
    AssetCollection, AssetDiffEngine, AssetCompressor, CompressedAssetCollection
};
use tracing::info;
use std::path::PathBuf;

mod config;
use config::Config;

#[cfg(test)]
mod cli_tests;

#[derive(Parser)]
#[command(name = "rodepush")]
#[command(about = "A React Native CodePush CLI tool")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Path to configuration file
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Parser)]
enum Commands {
    /// Build a React Native bundle
    Build {
        /// Path to the React Native project directory
        #[arg(long)]
        project_dir: Option<PathBuf>,
        
        /// Path to the assets directory
        #[arg(long)]
        assets_dir: Option<PathBuf>,
        
        /// Platform to build for (ios, android, both)
        #[arg(long, default_value = "both")]
        platform: String,
        
        /// Entry file for the React Native bundle
        #[arg(long)]
        entry_file: Option<String>,
        
        /// Output directory for built bundles
        #[arg(long)]
        output_dir: Option<String>,
    },
    /// Upload a bundle to the server
    Upload {
        /// Path to the bundle file
        #[arg(long)]
        bundle_path: Option<PathBuf>,
        
        /// Path to the assets directory
        #[arg(long)]
        assets_dir: Option<PathBuf>,
        
        /// Server URL
        #[arg(long)]
        server_url: Option<String>,
        
        /// API key for authentication
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Deploy a bundle to an environment
    Deploy {
        /// Application ID
        #[arg(long)]
        app_id: Option<String>,
        
        /// Environment to deploy to
        #[arg(long)]
        environment: Option<String>,
    },
    
    /// Process assets (create collection, diff, compress)
    Assets {
        #[command(subcommand)]
        action: AssetActions,
    },
}

#[derive(Parser)]
enum AssetActions {
    /// Create an asset collection from a directory
    Create {
        /// Path to the assets directory
        assets_dir: PathBuf,
        
        /// Output file for the asset collection (JSON format)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    
    /// Diff two asset collections
    Diff {
        /// Path to the old asset collection JSON file
        old_collection: PathBuf,
        
        /// Path to the new asset collection JSON file
        new_collection: PathBuf,
        
        /// Output file for the diff result (JSON format)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    
    /// Compress an asset collection
    Compress {
        /// Path to the asset collection JSON file
        collection: PathBuf,
        
        /// Output file for the compressed data
        #[arg(long)]
        output: Option<PathBuf>,
    },
    
    /// Decompress an asset collection
    Decompress {
        /// Path to the compressed asset collection file
        compressed_collection: PathBuf,
        
        /// Output directory for the decompressed assets
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        Config::from_file(config_path)?
    } else {
        Config::load()?
    };

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    let log_config = LogConfig {
        level: log_level.to_string(),
        format: LogFormat::Text,
        ..Default::default()
    };
    
    if let Err(e) = init_logging(&log_config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    let context = LogContext::new("main", "rodepush-cli");

    match &cli.command {
        Some(Commands::Build { project_dir, assets_dir, platform, entry_file, output_dir }) => {
            context.info("Building React Native bundle");
            
            // Use config values as defaults, override with command line args
            let effective_platform = if platform != "both" { 
                platform 
            } else { 
                &config.build.platform 
            };
            
            let effective_entry_file = entry_file.as_ref().unwrap_or(&config.build.entry_file);
            let effective_output_dir = output_dir.as_ref().unwrap_or(&config.build.output_dir);
            
            info!("Build command executed with project_dir: {:?}, assets_dir: {:?}, platform: {}, entry_file: {}, output_dir: {}", 
                  project_dir, assets_dir, effective_platform, effective_entry_file, effective_output_dir);
            
            // If assets directory is provided, create an asset collection
            if let Some(assets_path) = assets_dir {
                if assets_path.exists() && assets_path.is_dir() {
                    context.info(&format!("Processing assets from: {:?}", assets_path));
                    let asset_collection = AssetCollection::from_directory(assets_path)?;
                    println!("Created asset collection with {} assets, total size: {} bytes", 
                             asset_collection.len(), asset_collection.total_size);
                } else {
                    eprintln!("Assets directory does not exist or is not a directory: {:?}", assets_path);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Upload { bundle_path, assets_dir, server_url, api_key }) => {
            context.info("Uploading bundle to server");
            
            // Use config values as defaults, override with command line args
            let effective_server_url = server_url.as_ref().unwrap_or(&config.server.url);
            // For API key, we would normally load from file or env, but for now we'll just use a placeholder
            let _effective_api_key = api_key.as_deref().or_else(|| {
                // In a real implementation, we'd load from config.auth.api_key_file
                None
            }).unwrap_or("placeholder-api-key");
            
            info!("Upload command executed with bundle_path: {:?}, assets_dir: {:?}, server_url: {}, api_key: ***", 
                  bundle_path, assets_dir, effective_server_url);
            
            // If assets directory is provided, process it
            if let Some(assets_path) = assets_dir {
                if assets_path.exists() && assets_path.is_dir() {
                    context.info(&format!("Processing assets from: {:?}", assets_path));
                    let asset_collection = AssetCollection::from_directory(assets_path)?;
                    println!("Processed asset collection with {} assets", asset_collection.len());
                } else {
                    eprintln!("Assets directory does not exist or is not a directory: {:?}", assets_path);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Deploy { app_id, environment }) => {
            context.info("Deploying bundle to environment");
            
            let effective_app_id = app_id.as_deref().unwrap_or("default-app");
            let effective_environment = environment.as_deref().unwrap_or("production");
            
            info!("Deploy command executed with app_id: {}, environment: {}", 
                  effective_app_id, effective_environment);
        }
        Some(Commands::Assets { action }) => {
            match action {
                AssetActions::Create { assets_dir, output } => {
                    context.info(&format!("Creating asset collection from: {:?}", assets_dir));
                    if assets_dir.exists() && assets_dir.is_dir() {
                        let asset_collection = AssetCollection::from_directory(assets_dir)?;
                        println!("Created asset collection with {} assets, total size: {} bytes", 
                                 asset_collection.len(), asset_collection.total_size);
                        
                        if let Some(output_path) = output {
                            let json = serde_json::to_string_pretty(&asset_collection)?;
                            std::fs::write(output_path, json)?;
                            println!("Asset collection saved to: {:?}", output_path);
                        }
                    } else {
                        eprintln!("Assets directory does not exist or is not a directory: {:?}", assets_dir);
                        std::process::exit(1);
                    }
                }
                AssetActions::Diff { old_collection, new_collection, output } => {
                    context.info("Computing asset diff");
                    let old_json = std::fs::read_to_string(old_collection)?;
                    let new_json = std::fs::read_to_string(new_collection)?;
                    
                    let old_collection: AssetCollection = serde_json::from_str(&old_json)?;
                    let new_collection: AssetCollection = serde_json::from_str(&new_json)?;
                    
                    let diff_engine = AssetDiffEngine::new();
                    let diff = diff_engine.diff(&old_collection, &new_collection)?;
                    
                    println!("Asset diff computed:");
                    println!("  Added: {}", diff.added.len());
                    println!("  Removed: {}", diff.removed.len());
                    println!("  Renamed: {}", diff.renamed.len());
                    println!("  Modified: {}", diff.modified.len());
                    
                    if let Some(output_path) = output {
                        let json = serde_json::to_string_pretty(&diff)?;
                        std::fs::write(output_path, json)?;
                        println!("Asset diff saved to: {:?}", output_path);
                    }
                }
                AssetActions::Compress { collection, output } => {
                    context.info("Compressing asset collection");
                    let json = std::fs::read_to_string(collection)?;
                    let asset_collection: AssetCollection = serde_json::from_str(&json)?;
                    
                    let compressed = AssetCompressor::compress_collection(&asset_collection)?;
                    
                    println!("Asset collection compressed:");
                    println!("  Uncompressed size: {} bytes", compressed.uncompressed_size);
                    println!("  Compressed size: {} bytes", compressed.compressed_size);
                    println!("  Compression ratio: {:.2}%", 
                             (1.0 - (compressed.compressed_size as f64 / compressed.uncompressed_size as f64)) * 100.0);
                    
                    if let Some(output_path) = output {
                        std::fs::write(output_path, &compressed.data)?;
                        println!("Compressed asset collection saved to: {:?}", output_path);
                    }
                }
                AssetActions::Decompress { compressed_collection, output_dir } => {
                    context.info("Decompressing asset collection");
                    let compressed_data = std::fs::read(compressed_collection)?;
                    
                    let compressed_collection = CompressedAssetCollection {
                        data: compressed_data,
                        uncompressed_size: 0, // These values aren't used in decompression
                        compressed_size: 0,   // These values aren't used in decompression
                        compression_type: rodepush_core::CompressionType::Zstd,
                    };
                    
                    let asset_collection = AssetCompressor::decompress_collection(&compressed_collection)?;
                    
                    println!("Asset collection decompressed with {} assets", asset_collection.len());
                    
                    if let Some(output_path) = output_dir {
                        // In a real implementation, we would extract the assets to the directory
                        // For now, we'll just save the collection metadata
                        let json = serde_json::to_string_pretty(&asset_collection)?;
                        std::fs::write(output_path.join("assets.json"), json)?;
                        println!("Decompressed asset collection metadata saved to: {:?}", output_path);
                    }
                }
            }
        }
        None => {
            println!("RodePush CLI - Use --help for available commands");
            info!("CLI started without command");
        }
    }

    Ok(())
}