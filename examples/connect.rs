//! Example: Connect to an NTRIP caster and stream RTCM data.
//!
//! Usage:
//!   cargo run --example connect -- <host> <mountpoint> [port] [--user=X] [--pass=X] [--tls]
//!
//! Example:
//!   cargo run --example connect -- rtk2go.com Laguna01 2101 --user=user@example.com --pass=none
//!   cargo run --example connect -- auscors.ga.gov.au ALIC00AUS0 443 --tls

use ntrip_core::{Error, NtripClient, NtripConfig};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: {} <host> <mountpoint> [port] [--user=X] [--pass=X] [--tls]",
            args[0]
        );
        eprintln!();
        eprintln!("Examples:");
        eprintln!(
            "  {} rtk2go.com Laguna01 2101 --user=user@example.com --pass=none",
            args[0]
        );
        eprintln!("  {} auscors.ga.gov.au ALIC00AUS0 443 --tls", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let mountpoint = &args[2];
    let port: u16 = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(2101);

    // Parse optional credentials and flags
    let user = args.iter().find_map(|a| a.strip_prefix("--user="));
    let pass = args.iter().find_map(|a| a.strip_prefix("--pass="));
    let use_tls = args.iter().any(|a| a == "--tls" || a == "--https");

    println!(
        "Connecting to {}:{}/{}{}",
        host,
        port,
        mountpoint,
        if use_tls { " (TLS)" } else { "" }
    );

    // Build configuration
    let mut config = NtripConfig::new(host, port, mountpoint);
    if let (Some(u), Some(p)) = (user, pass) {
        config = config.with_credentials(u, p);
    }
    if use_tls {
        config = config.with_tls();
    }

    // Create client and connect
    let mut client = NtripClient::new(config)?;
    client.connect().await?;

    println!("Connected! Reading RTCM data for 10 seconds...\n");

    // Read data for 10 seconds
    let mut buf = [0u8; 4096];
    let mut total_bytes = 0usize;
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(10) {
        match tokio::time::timeout(Duration::from_secs(2), client.read_chunk(&mut buf)).await {
            Ok(Ok(n)) => {
                total_bytes += n;
                // Show first few bytes as hex
                print!("[{:4} bytes] ", n);
                for b in &buf[..n.min(16)] {
                    print!("{:02X} ", b);
                }
                if n > 16 {
                    print!("...");
                }
                println!();
            }
            Ok(Err(e)) => {
                eprintln!("Error: {}", e);
                break;
            }
            Err(_) => {
                // Timeout, continue
            }
        }
    }

    println!("\nReceived {} bytes in {:?}", total_bytes, start.elapsed());
    println!(
        "Average rate: {:.1} bytes/sec",
        total_bytes as f64 / start.elapsed().as_secs_f64()
    );

    Ok(())
}
