//! # Runestone Web UI Engine
//!
//! `rune-web` provides an embedded, local-only, read-only web interface for browsing
//! Rune repository commit timelines, file trees, syntax-highlighted blobs, and unified diffs.
//!
//! ## Security Model
//! - Binds exclusively to loopback (`127.0.0.1`).
//! - Strictly read-only (`GET` endpoints only).
//! - Enforces anti-DNS rebinding and origin validation middleware ([`middleware::validate_host`]).

pub mod middleware;
pub mod routes;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use axum::middleware::from_fn;

/// Shared application state holding the root repository path.
#[derive(Clone)]
pub struct AppState {
    /// Path to the root of the Rune repository being browsed.
    pub repo_path: PathBuf,
}

/// Constructs the Axum [`Router`] for Runestone with all registered read routes
/// and host validation middleware.
///
/// # Arguments
/// * `repo_path` - Path to the root of the target Rune repository.
pub fn app(repo_path: PathBuf) -> Router {
    let state = AppState { repo_path };

    Router::new()
        .route("/", get(routes::index))
        .route("/commits", get(routes::commits::handler))
        .route("/tree", get(routes::tree::head_handler))
        .route("/commit/:hash/tree", get(routes::tree::root_handler))
        .route("/commit/:hash/tree/*path", get(routes::tree::handler))
        .route("/commit/:hash/blob/*path", get(routes::blob::handler))
        .route("/commit/:hash/diff", get(routes::diff::handler))
        .route("/refs", get(routes::refs::handler))
        // Apply host validation middleware to all routes
        .layer(from_fn(middleware::validate_host))
        .with_state(state)
}

/// Starts the Runestone web server bound to `127.0.0.1:<port>`.
///
/// # Arguments
/// * `port` - Local TCP port to bind (default: 8080).
/// * `repo_path` - Path to the root of the target Rune repository.
///
/// # Errors
/// Returns an error if TCP binding fails or if the Axum server crashes.
pub async fn start_server(port: u16, repo_path: PathBuf) -> anyhow::Result<()> {
    let app = app(repo_path);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Runestone starting on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
