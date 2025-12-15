use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use git_version::git_version;
use log::error;
use std::fmt::Write;
use sysinfo::{RefreshKind, System};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

async fn push_metrics(push_http_path: &Path, sys: &System) -> io::Result<()> {
    let mut metrics = String::new();
    let _ = metrics.write_str(
        "# HELP node_global_cpu_usage Global CPUs usage (aka the addition of all the CPUs)
# TYPE node_global_cpu_usage gauge
",
    );
    let _ = metrics.write_fmt(format_args!(
        "node_global_cpu_usage {}\n",
        sys.global_cpu_usage()
    ));
    let _ = metrics.write_str("# EOF\n");

    let request = format!(
        "POST /metrics/job/node HTTP/1.1
Host: localhost
User-Agent: node-exporter/{}
Accept: */*
Content-Length: {}
Content-Type: application/openmetrics-text; version=1.0.0; charset=utf-8
Connection: close

{}",
        git_version!(),
        metrics.len(),
        metrics
    );

    let mut stream = UnixStream::connect(push_http_path).await?;
    stream.write_all(request.as_bytes()).await?;

    let mut tmp = [0u8; 100];
    let timeout = Duration::from_secs(5);

    loop {
        match tokio::time::timeout(timeout, stream.read(&mut tmp)).await {
            Ok(Ok(0)) => break,          // EOF
            Ok(Ok(_)) => continue,       // Read some bytes, keep going
            Ok(Err(e)) => return Err(e), // Read error
            Err(_) => break,             // Timed out
        }
    }

    Ok(())
}

pub async fn task(metrics_interval: Duration, push_http_path: PathBuf) -> io::Result<()> {
    let mut sys = System::new_with_specifics(RefreshKind::nothing());

    loop {
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        if let Err(e) = push_metrics(&push_http_path, &sys).await {
            error!("Problem writing metrics: {e}");
        }

        tokio::time::sleep(metrics_interval).await;
    }
}
