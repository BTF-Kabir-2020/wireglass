#![cfg(feature = "server")]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{Json, Router, extract::State, http::StatusCode, routing::get, routing::post};
use serde::Deserialize;
use tokio::sync::Semaphore;
use wireglass::{run, ssrf};

#[derive(Clone)]
struct AppState {
    cache: Arc<Mutex<HashMap<String, (Instant, String)>>>,
    sem: Arc<Semaphore>,
}

#[derive(Deserialize)]
struct ScanRequest {
    url: String,
    #[serde(default = "default_timeout")]
    timeout: u64,
}

fn default_timeout() -> u64 {
    10
}

async fn health() -> &'static str {
    "ok"
}

async fn scan(State(state): State<AppState>, Json(req): Json<ScanRequest>) -> (StatusCode, String) {
    let url = req.url;
    if let Err(e) = ssrf::validate_url(&url).await {
        return (StatusCode::BAD_REQUEST, e);
    }

    // Cache hit.
    {
        let cache = state.cache.lock().unwrap();
        if let Some((t, body)) = cache.get(&url)
            && t.elapsed() < Duration::from_secs(60)
        {
            return (StatusCode::OK, body.clone());
        }
    }

    // Concurrency cap (rate limiting by queueing).
    let permit = match tokio::time::timeout(Duration::from_secs(2), state.sem.acquire()).await {
        Ok(Ok(p)) => p,
        _ => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "too many in-flight requests".to_string(),
            );
        }
    };
    let body = {
        let report = run(&url, req.timeout, 1).await;
        serde_json::to_string_pretty(&report).unwrap_or_default()
    };
    drop(permit);

    // Store in cache.
    {
        let mut cache = state.cache.lock().unwrap();
        cache.insert(url, (Instant::now(), body.clone()));
    }

    (StatusCode::OK, body)
}

#[tokio::main]
async fn main() {
    let state = AppState {
        cache: Arc::new(Mutex::new(HashMap::new())),
        sem: Arc::new(Semaphore::new(4)),
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/scan", post(scan))
        .with_state(state);
    let port = std::env::var("WIREGLASS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    println!("wireglassd listening on http://127.0.0.1:{port}");
    axum::serve(listener, app).await.unwrap();
}
