use std::net::SocketAddr;

use axum::{
  Json,
  Router,
  routing::{get, post},
};
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};

async fn health() -> Json<Value> {
  Json(json!({ "status": "ok" }))
}

async fn receive_webhook(Json(payload): Json<Value>) -> Json<Value> {
  tracing::info!(payload = ?payload, "received webhook");
  Json(json!({ "received": true }))
}

#[tokio::main]
async fn main() {
  fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

  let port = std::env::var("PORT")
    .ok()
    .and_then(|value| value.parse::<u16>().ok())
    .unwrap_or(5000);

  let app = Router::new()
    .route("/health", get(health))
    .route("/webhook", post(receive_webhook))
    .layer(TraceLayer::new_for_http());

  let addr = SocketAddr::from(([0, 0, 0, 0], port));
  let listener = tokio::net::TcpListener::bind(addr)
    .await
    .expect("failed to bind tcp listener");

  axum::serve(listener, app)
    .await
    .expect("failed to start webhook server");
}
