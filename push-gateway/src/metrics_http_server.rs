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
use tokio_stream::Stream;

use crate::metrics_cache::MetricsCache;

#[derive(Clone)]
struct RouteState {
    metrics_cache: MetricsCache,
}

async fn metrics_handler(State(state): State<RouteState>) -> impl IntoResponse {
    let stream_body: pin::Pin<Box<dyn Stream<Item = Result<String, io::Error>> + Send>> =
        Box::pin(stream! {
            for (_k, v) in state.metrics_cache.iter() {
                yield Ok(v);
            }
        });

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
    metrics_cache: MetricsCache,
) -> Result<(), io::Error> {
    let state = RouteState { metrics_cache };
    let router = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(metrics_http_addr).await?;

    info!("Metrics HTTP listening on {}", metrics_http_addr);
    axum::serve(listener, router).await?;

    Ok(())
}
