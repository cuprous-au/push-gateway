use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use git_version::git_version;
use log::error;
use std::fmt::Write;
use sysinfo::{Components, Disks, Networks, System};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

async fn push_metrics(
    push_http_path: &Path,
    sys: &System,
    disks: &Disks,
    components: &Components,
    networks: &Networks,
) -> io::Result<()> {
    let mut metrics = String::new();
    let _ = metrics.write_str(
        "# HELP node_global_cpu_usage Global CPUs usage (aka the addition of all the CPUs)
# TYPE node_global_cpu_usage gauge
# HELP node_physical_core_count The combined the physical core count of all the CPUs
# TYPE node_physical_core_count gauge
# HELP node_load_average1 1 minute CPU load average
# TYPE node_load_average1 gauge
# HELP node_load_average5 5 minute CPU load average
# TYPE node_load_average5 gauge
# HELP node_load_average15 15 minute CPU load average
# TYPE node_load_average15 gauge
# HELP node_total_memory The RAM size in bytes
# TYPE node_total_memory gauge
# HELP node_available_memory The amount of available RAM in bytes including reusable memory
# TYPE node_available_memory gauge
# HELP node_disk_total_space The total disk size, in bytes
# TYPE node_disk_total_space gauge
# HELP node_disk_available_space The available disk size, in bytes
# TYPE node_disk_available_space gauge
# HELP node_temperature The temperature of the component (in celsius)
# TYPE node_temperature gauge
# HELP node_total_packets_received The total number of incoming packets
# TYPE node_total_packets_received gauge
# HELP node_total_packets_transmitted The total number of outgoing packets
# TYPE node_total_packets_transmitted gauge
",
    );

    let _ = metrics.write_fmt(format_args!(
        "node_global_cpu_usage {}\n",
        sys.global_cpu_usage()
    ));

    let _ = metrics.write_fmt(format_args!(
        "node_physical_core_count {}\n",
        System::physical_core_count().unwrap_or_default()
    ));

    let load_average = System::load_average();
    let _ = metrics.write_fmt(format_args!("node_load_average1 {}\n", load_average.one));
    let _ = metrics.write_fmt(format_args!("node_load_average5 {}\n", load_average.five));
    let _ = metrics.write_fmt(format_args!(
        "node_load_average15 {}\n",
        load_average.fifteen
    ));

    let _ = metrics.write_fmt(format_args!("node_total_memory {}\n", sys.total_memory()));
    let _ = metrics.write_fmt(format_args!(
        "node_available_memory {}\n",
        sys.available_memory()
    ));

    for disk in disks {
        if !disk.is_read_only() && !disk.is_removable() {
            let name = disk.name().to_string_lossy();
            let _ = metrics.write_fmt(format_args!(
                "node_disk_total_space{{name=\"{}\"}} {}\n",
                name,
                disk.total_space()
            ));
            let _ = metrics.write_fmt(format_args!(
                "node_disk_available_space{{name=\"{}\"}} {}\n",
                name,
                disk.available_space()
            ));
        }
    }

    for component in components {
        if let Some(temperature) = component.temperature() {
            let _ = metrics.write_fmt(format_args!(
                "node_temperature{{id=\"{}\"}} {}\n",
                component.id().unwrap_or("unavailable"),
                temperature
            ));
        }
    }

    for (interface, network) in networks {
        if interface.starts_with("en") || interface.starts_with("eth") {
            let _ = metrics.write_fmt(format_args!(
                "node_total_packets_received{{interface=\"{}\"}} {}\n",
                interface,
                network.total_packets_received()
            ));
            let _ = metrics.write_fmt(format_args!(
                "node_total_packets_transmitted{{interface=\"{}\"}} {}\n",
                interface,
                network.total_packets_transmitted()
            ));
        }
    }

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
    let mut sys = System::new();
    let mut disks = Disks::new();
    let mut components = Components::new();
    let mut networks = Networks::new();

    loop {
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        disks.refresh(false);
        components.refresh(false);
        networks.refresh(false);

        if let Err(e) = push_metrics(&push_http_path, &sys, &disks, &components, &networks).await {
            error!("Problem writing metrics: {e}");
        }

        tokio::time::sleep(metrics_interval).await;
    }
}
