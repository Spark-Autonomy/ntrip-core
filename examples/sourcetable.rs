//! Example: Fetch and display a caster's sourcetable.
//!
//! Usage:
//!   cargo run --example sourcetable -- <host> [port] [--user=X] [--pass=X] [--tls]
//!
//! Examples:
//!   cargo run --example sourcetable -- rtk2go.com
//!   cargo run --example sourcetable -- ntrip.data.gnss.ga.gov.au 443 --tls

use ntrip_core::{Error, NtripClient, NtripConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <host> [port] [--user=X] [--pass=X] [--tls]",
            args[0]
        );
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} rtk2go.com", args[0]);
        eprintln!(
            "  {} ntrip.data.gnss.ga.gov.au 443 --user=USER --pass=PASS",
            args[0]
        );
        std::process::exit(1);
    }

    let host = &args[1];
    let port: u16 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(2101);

    // Parse optional credentials
    let user = args.iter().find_map(|a| a.strip_prefix("--user="));
    let pass = args.iter().find_map(|a| a.strip_prefix("--pass="));
    let use_tls = args.iter().any(|a| a == "--tls" || a == "--https");

    println!("Fetching sourcetable from {}:{}...\n", host, port);

    // Build configuration
    let mut config = NtripConfig::new(host, port, "");
    if let (Some(u), Some(p)) = (user, pass) {
        config = config.with_credentials(u, p);
    }
    if use_tls {
        config = config.with_tls();
    }

    // Fetch sourcetable
    let table = NtripClient::get_sourcetable(&config).await?;

    // Display summary
    println!("=== Sourcetable Summary ===");
    println!(
        "Streams: {}  Casters: {}  Networks: {}\n",
        table.streams.len(),
        table.casters.len(),
        table.networks.len()
    );

    // Display casters
    if !table.casters.is_empty() {
        println!("--- Casters ---");
        for caster in &table.casters {
            println!("  {}:{} - {}", caster.host, caster.port, caster.identifier);
        }
        println!();
    }

    // Display networks
    if !table.networks.is_empty() {
        println!("--- Networks ---");
        for network in &table.networks {
            println!("  {} - {}", network.identifier, network.operator);
        }
        println!();
    }

    // Display RTCM streams (first 20)
    let rtcm_streams = table.rtcm_streams();
    println!("--- RTCM Streams ({} total) ---", rtcm_streams.len());
    println!(
        "{:<20} {:<15} {:<20} {:>10} {:>10}",
        "Mountpoint", "Format", "NavSys", "Lat", "Lon"
    );
    println!("{:-<80}", "");

    for stream in rtcm_streams.iter().take(20) {
        println!(
            "{:<20} {:<15} {:<20} {:>10.4} {:>10.4}",
            truncate(&stream.mountpoint, 20),
            truncate(&stream.format, 15),
            truncate(&stream.nav_system, 20),
            stream.latitude,
            stream.longitude
        );
    }

    if rtcm_streams.len() > 20 {
        println!("  ... and {} more streams", rtcm_streams.len() - 20);
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
