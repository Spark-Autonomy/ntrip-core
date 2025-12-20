//! Example: Find the nearest RTCM mountpoint to a given location.
//!
//! Usage:
//!   cargo run --example nearest -- <host> <lat> <lon> [port] [--tls]
//!
//! Examples:
//!   cargo run --example nearest -- rtk2go.com -27.47 153.02
//!   cargo run --example nearest -- ntrip.data.gnss.ga.gov.au -27.47 153.02 443 --tls

use ntrip_core::{Error, NtripClient, NtripConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <host> <lat> <lon> [port] [--tls]", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} rtk2go.com -27.47 153.02", args[0]);
        eprintln!("  {} rtk2go.com 40.71 -74.00  # New York", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let lat: f64 = args[2].parse().expect("Invalid latitude");
    let lon: f64 = args[3].parse().expect("Invalid longitude");
    let port: u16 = args.get(4).and_then(|p| p.parse().ok()).unwrap_or(2101);

    println!(
        "Finding nearest RTCM mountpoint to ({}, {}) from {}:{}...\n",
        lat, lon, host, port
    );

    // Parse optional flags
    let use_tls = args.iter().any(|a| a == "--tls" || a == "--https");

    // Fetch sourcetable
    let mut config = NtripConfig::new(host, port, "");
    if use_tls {
        config = config.with_tls();
    }
    let table = NtripClient::get_sourcetable(&config).await?;

    // Find nearest streams
    println!("Top 10 nearest RTCM streams:");
    println!("{:-<80}", "");
    println!(
        "{:>3}  {:<20} {:>10} km  Format/NavSys",
        "#", "Mountpoint", "Distance"
    );
    println!("{:-<80}", "");

    let streams = table.streams_by_distance(lat, lon);
    let mut count = 0;

    for (stream, dist) in streams.iter() {
        if stream.is_rtcm() {
            count += 1;
            println!(
                "{:>3}. {:<20} {:>10.1} km  {} ({})",
                count, stream.mountpoint, dist, stream.format, stream.nav_system
            );
            if count >= 10 {
                break;
            }
        }
    }

    // Recommendation
    if let Some((nearest, dist)) = table.nearest_rtcm_stream(lat, lon) {
        println!();
        println!("Recommended: {} at {:.1} km", nearest.mountpoint, dist);
        println!("  Format: {}", nearest.format);
        println!("  NavSys: {}", nearest.nav_system);
        println!(
            "  Position: ({:.4}, {:.4})",
            nearest.latitude, nearest.longitude
        );
    }

    Ok(())
}
