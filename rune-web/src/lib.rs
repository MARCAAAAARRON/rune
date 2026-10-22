pub mod middleware;
pub mod routes;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use axum::middleware::from_fn;

#[derive(Clone)]
pub struct AppState {
    pub repo_path: PathBuf,
}

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

pub async fn start_server(port: u16, repo_path: PathBuf) -> anyhow::Result<()> {
    let app = app(repo_path);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Runestone starting on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
