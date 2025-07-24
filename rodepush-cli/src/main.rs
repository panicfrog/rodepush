use clap::Parser;
use rodepush_core::{init_logging, LogContext, LogConfig, LogFormat};
use tracing::info;

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
}

#[derive(Parser)]
enum Commands {
    /// Build a React Native bundle
    Build,
    /// Upload a bundle to the server
    Upload,
    /// Deploy a bundle to an environment
    Deploy,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

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
        Some(Commands::Build) => {
            context.info("Building React Native bundle");
            info!("Build command executed");
        }
        Some(Commands::Upload) => {
            context.info("Uploading bundle to server");
            info!("Upload command executed");
        }
        Some(Commands::Deploy) => {
            context.info("Deploying bundle to environment");
            info!("Deploy command executed");
        }
        None => {
            println!("RodePush CLI - Use --help for available commands");
            info!("CLI started without command");
        }
    }

    Ok(())
}