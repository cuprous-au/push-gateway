#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;
use std::{error::Error, io, net::SocketAddr};

use clap::Parser;
use git_version::git_version;
use log::info;
use moka::policy::EvictionPolicy;
use tokio::task::JoinError;

use crate::metrics_cache::FamiliesCache;

mod metrics_cache;
mod metrics_http_server;
mod push_http_server;

/// A push acceptor for caching the Prometheus metrics of local processes,
#[derive(Parser, Debug)]
#[clap(author, about, long_about = None, version=git_version!())]
pub struct Args {
    /// The total number of families.
    #[clap(env, long, default_value_t = 1)]
    max_families: u64,

    /// The total number of metrics to retain per family.
    #[clap(env, long, default_value_t = 100)]
    max_metrics_per_family: u64,

    /// A socket address for serving our families of metrics. Delivers
    /// application/openmetrics-text; version=1.0.0; as a content type.
    /// Defaults to localhost.
    #[clap(env, long, default_value = "127.0.0.1:9091")]
    metrics_http_addr: SocketAddr,

    /// A unix socket path to bind to for serving our http push requests.
    #[clap(env, long, default_value = "/var/run/push-gateway.sock")]
    push_http_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    env_logger::builder().format_timestamp_millis().init();

    // Establish the cache of metrics

    let families_cache = FamiliesCache::builder()
        .max_capacity(args.max_families)
        .eviction_policy(EvictionPolicy::lru())
        .build();

    // Startup a metrics endpoint

    let metrics_http_server = tokio::spawn(metrics_http_server::task(
        args.metrics_http_addr,
        families_cache.clone(),
    ));

    // Startup a push endpoint

    let push_http_server = tokio::spawn(push_http_server::task(
        args.push_http_path,
        families_cache,
        args.max_metrics_per_family,
    ));

    // All things started. Wait for the important tasks complete.

    info!("Push gateway ready");

    fn flatten(r: Result<Result<(), io::Error>, JoinError>) -> Result<(), Box<dyn Error>> {
        match r {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => Err(e.into()),
        }
    }

    tokio::select! {
        r = metrics_http_server => {
            flatten(r)
        }
        r = push_http_server => {
            flatten(r)
        }
    }
}
