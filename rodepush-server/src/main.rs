use axum::{
    routing::get,
    Router,
};
use rodepush_core::{init_server_logging, LogContext};
use tracing::{info, error};
use tower_http::trace::TraceLayer;

async fn hello() -> &'static str {
    let context = LogContext::new("hello_handler", "rodepush-server");
    context.info("Hello endpoint accessed");
    "RodePush Server - Coming soon!"
}

async fn health() -> &'static str {
    let context = LogContext::new("health_check", "rodepush-server");
    context.debug("Health check endpoint accessed");
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    if let Err(e) = init_server_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    let context = LogContext::new("server_startup", "rodepush-server");
    context.info("Starting RodePush server");

    let app = Router::new()
        .route("/", get(hello))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("RodePush server listening on http://0.0.0.0:8080");
    
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}