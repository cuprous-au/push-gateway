use std::{
    io::{self},
    path::PathBuf,
};

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use log::info;
use serde::Deserialize;

use crate::metrics_cache::MetricsCache;

#[derive(Clone)]
struct RouteState {
    _metrics_cache: MetricsCache,
}

async fn push_handler_with_job(
    State(state): State<RouteState>,
    Path(job): Path<String>,
) -> StatusCode {
    push_handler(state, job, vec![]).await
}

#[derive(Deserialize)]
struct JobWithLabels {
    job: String,
    labels: String,
}

async fn push_handler_with_job_and_labels(
    State(state): State<RouteState>,
    Path(path): Path<JobWithLabels>,
) -> StatusCode {
    let labels = path
        .labels
        .split("/")
        .fold(
            (Vec::new(), None),
            |(mut labels, mut prev_key), path_item| {
                if let Some(key) = prev_key {
                    labels.push((key, path_item.to_string()));
                    prev_key = None;
                } else {
                    prev_key = Some(path_item.to_string());
                }
                (labels, prev_key)
            },
        )
        .0;
    push_handler(state, path.job, labels).await
}

async fn push_handler(
    _state: RouteState,
    _job: String,
    _labels: Vec<(String, String)>,
) -> StatusCode {
    StatusCode::OK
}

pub(crate) async fn task(
    push_http_path: PathBuf,
    metrics_cache: MetricsCache,
) -> Result<(), io::Error> {
    let state = RouteState {
        _metrics_cache: metrics_cache,
    };
    let router = Router::new()
        .nest(
            "/metrics",
            Router::new().nest(
                "/job/{job}",
                Router::new()
                    .route("/{*labels}", post(push_handler_with_job_and_labels))
                    .route("/", post(push_handler_with_job)),
            ),
        )
        .with_state(state);
    let _ = std::fs::remove_file(&push_http_path);
    let listener = tokio::net::UnixListener::bind(&push_http_path)?;

    info!(
        "Push HTTP listening on {}",
        push_http_path.to_string_lossy()
    );
    axum::serve(listener, router).await?;

    Ok(())
}
