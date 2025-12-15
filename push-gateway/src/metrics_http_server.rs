use std::{
    io::{self},
    pin,
};

use async_stream::stream;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::get,
};
use log::info;
use std::fmt::Write;
use tokio_stream::Stream;

use crate::metrics_cache::FamiliesCache;

#[derive(Clone)]
struct RouteState {
    families_cache: FamiliesCache,
}

async fn metrics_handler(State(state): State<RouteState>) -> impl IntoResponse {
    let stream_body: pin::Pin<Box<dyn Stream<Item = Result<String, io::Error>> + Send>> = Box::pin(
        stream! {
            for (families_k, families_v) in state.families_cache.iter() {
                yield Ok(families_v.descriptors);

                let mut job_labels = String::new();
                let _ = job_labels.write_fmt(format_args!("job={}", families_k.job));
                for (label_k, label_v) in families_k.labels.iter() {
                    let _ = job_labels.write_fmt(format_args!(",{}={}", label_k, label_v));
                }

                for (metrics_k, metric) in families_v.metrics_cache.iter() {
                    let mut metric_labelled_name = String::with_capacity(job_labels.len() + metrics_k.name.len());
                    let _ = metric_labelled_name.write_str(&metrics_k.name);
                    let _ = metric_labelled_name.write_char('{');
                    let _ = metric_labelled_name.write_str(&job_labels);
                    for (name, value) in &metrics_k.labels {
                        let _ = metric_labelled_name.write_fmt(format_args!(",{}={}", name, value));
                    }
                    let _ = metric_labelled_name.write_char('}');

                    yield Ok(format!("{metric_labelled_name} {metric}\n"));
                }
            }
        },
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(Body::from_stream(stream_body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub(crate) async fn task(
    metrics_http_addr: std::net::SocketAddr,
    families_cache: FamiliesCache,
) -> Result<(), io::Error> {
    let state = RouteState { families_cache };
    let router = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(metrics_http_addr).await?;

    info!("Metrics HTTP listening on {}", metrics_http_addr);
    axum::serve(listener, router).await?;

    Ok(())
}
