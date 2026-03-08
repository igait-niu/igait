// iGait Backend - Microservice Orchestrator
// Handles job uploads, stage dispatching, and result notifications
// Force rebuild

mod helper;
mod routes;

use anyhow::{ Context, Result };
use axum::{
    extract::DefaultBodyLimit, routing::{any, get, post}, Router
};
use helper::lib::{AppState, AppStatePtr};
use std::sync::Arc;
use dotenv::dotenv;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const ASD_CLASSIFICATION_THRESHOLD: f32 = 0.5;
pub const DISABLE_RESULT_EMAIL: bool = true;

/// The main entrypoint for the iGait backend.
/// 
/// # Fails
/// * If the app state fails to initialize
/// * If the listener fails to start
/// * If the API fails to serve
/// 
/// # Returns
/// * A successful result if the API is served
/// 
/// # Notes
/// * The API is served on port 3000
/// * The API is served at the root of the server
/// * The API is served with a body limit of 500MB
/// * The API is served with the V1 API nested under `/api/v1`
/// * Gracefully shuts down on SIGTERM or Ctrl+C
#[tokio::main]
async fn main() -> Result<()> {
    
    // Enable loading on WSL
    dotenv().ok();

    // Initialize tracing subscriber with environment filter
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "igait_backend=info,tower_http=info,axum::rejection=trace".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting iGait backend initialization...");

    // Create a thread-safe mutex lock to hold the app state
    let state: Arc<AppState> = Arc::new(
        AppState::new().await.context("Couldn't set up app state!")?
    );
    let app_state_ptr = AppStatePtr { state: state.clone() };

    // Build the V1 API router
    let api_v1 = Router::new()
        .route("/upload", post(crate::routes::upload::upload_entrypoint) )
        .route("/contribute", post(crate::routes::contribute::contribute_entrypoint))
        .route("/rerun", post(crate::routes::rerun::rerun_entrypoint))
        .route("/assistant", any(crate::routes::assistant::assistant_entrypoint))
        .route("/assistant_proxied", any(crate::routes::assistant::assistant_proxied_entrypoint))
        .route("/files/:job_id", get(crate::routes::files::files_entrypoint))
        .route("/cycles/:job_id", axum::routing::put(crate::routes::cycles::cycles_entrypoint))
        .route("/video-edit/:job_id", post(crate::routes::video_edit::video_edit_entrypoint))
        .with_state(app_state_ptr.clone());
    
    // Build the internal API router (for microservice communication)
    let api_internal = Router::new()
        .route("/update-status", post(crate::routes::internal::update_status))
        .with_state(app_state_ptr);

    // Nest the API into the general app router
    let app = Router::new()
        .nest("/api/v1", api_v1)
        .nest("/api/internal", api_internal)
        .layer(DefaultBodyLimit::max(500000000));

    // Setup graceful shutdown signal handling
    let shutdown_signal = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                warn!("Received Ctrl+C, shutting down gracefully...");
            },
            _ = terminate => {
                warn!("Received SIGTERM, shutting down gracefully...");
            },
        }
    };

    // Serve the API with graceful shutdown
    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    info!("iGait backend listening on port {}", port);
    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await
        .context("Couldn't start up listener!")?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .context("Could't serve the API!")?;

    info!("Server shut down gracefully");
    Ok(())
}
