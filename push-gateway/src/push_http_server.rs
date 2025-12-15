use std::{
    io::{self},
    path::PathBuf,
};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use futures::StreamExt;
use log::info;
use moka::policy::EvictionPolicy;
use nom_openmetrics::parser::family;
use serde::Deserialize;

use crate::metrics_cache::{FamiliesCache, FamiliesKey, FamiliesValue, MetricsCache, MetricsKey};

#[derive(Clone)]
struct RouteState {
    families_cache: FamiliesCache,
    max_metrics_per_family: u64,
}

async fn push_handler_with_job(
    State(state): State<RouteState>,
    Path(job): Path<String>,
    body: Body,
) -> StatusCode {
    push_handler(state, job, vec![], body).await
}

#[derive(Deserialize)]
struct JobWithLabels {
    job: String,
    labels: String,
}

async fn push_handler_with_job_and_labels(
    State(state): State<RouteState>,
    Path(path): Path<JobWithLabels>,
    body: Body,
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
    push_handler(state, path.job, labels, body).await
}

async fn push_handler(
    state: RouteState,
    job: String,
    labels: Vec<(String, String)>,
    body: Body,
) -> StatusCode {
    let mut stream_body = body.into_data_stream();

    const DEFAULT_METRICS_FAMILY_CAPACITY: usize = 1024;
    let mut buf = Vec::with_capacity(DEFAULT_METRICS_FAMILY_CAPACITY);

    let family_key = FamiliesKey::new(job, labels);
    let mut family_value = state
        .families_cache
        .get(&family_key)
        .unwrap_or_else(|| FamiliesValue {
            descriptors: String::new(),
            metrics_cache: MetricsCache::builder()
                .max_capacity(state.max_metrics_per_family)
                .eviction_policy(EvictionPolicy::lru())
                .build(),
        });
    while let Some(Ok(bytes)) = stream_body.next().await {
        buf.extend_from_slice(&bytes);

        let Ok(text) = std::str::from_utf8(&buf) else {
            continue;
        };

        match family(text) {
            Ok((remaining, metric_family)) => {
                family_value.descriptors.clear();
                for line in text.lines() {
                    if line.starts_with('#') {
                        family_value.descriptors.push_str(line);
                        family_value.descriptors.push('\n');
                    } else {
                        break;
                    }
                }

                for sample in metric_family.samples {
                    let metric_key =
                        MetricsKey::with_nom_name_and_labels(sample.name(), sample.labels());
                    family_value
                        .metrics_cache
                        .insert(metric_key, sample.number());
                }

                family_value.metrics_cache.run_pending_tasks();

                buf.drain(..buf.len() - remaining.len());
            }
            Err(nom::Err::Incomplete(_)) => {
                continue;
            }
            Err(_e) => {
                break;
            }
        }
    }

    state.families_cache.insert(family_key, family_value);
    state.families_cache.run_pending_tasks();

    StatusCode::OK
}

pub(crate) async fn task(
    push_http_path: PathBuf,
    families_cache: FamiliesCache,
    max_metrics_per_family: u64,
) -> Result<(), io::Error> {
    let state = RouteState {
        families_cache,
        max_metrics_per_family,
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
