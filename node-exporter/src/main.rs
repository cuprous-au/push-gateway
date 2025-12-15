#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;
use std::{error::Error, io};

use clap::Parser;
use git_version::git_version;
use log::info;
use tokio::task::JoinError;

mod push_http_client;

/// Exporter for machine metrics
#[derive(Parser, Debug)]
#[clap(author, about, long_about = None, version=git_version!())]
pub struct Args {
    /// The frequency of gathering metrics
    #[clap(env, long, default_value = "15s")]
    metrics_interval: humantime::Duration,

    /// A unix socket push http metrics to.
    #[clap(env, long, default_value = "/var/run/push-gateway.sock")]
    push_http_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    env_logger::builder().format_timestamp_millis().init();

    // Startup a push endpoint

    let push_http_client = tokio::spawn(push_http_client::task(
        args.metrics_interval.into(),
        args.push_http_path,
    ));

    // All things started. Wait for the important tasks complete.

    info!("Node exporter ready");

    fn flatten(r: Result<Result<(), io::Error>, JoinError>) -> Result<(), Box<dyn Error>> {
        match r {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => Err(e.into()),
        }
    }

    tokio::select! {
        r = push_http_client => {
            flatten(r)
        }
    }
}
