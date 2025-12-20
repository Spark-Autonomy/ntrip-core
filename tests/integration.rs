//! Integration tests against real NTRIP casters.
//!
//! These tests are ignored by default as they require network access.
//! Run with: cargo test --test integration -- --ignored

use ntrip_core::{NtripClient, NtripConfig, NtripVersion};
use std::time::Duration;

/// Test fetching sourcetable from RTK2go (public caster)
#[tokio::test]
#[ignore = "requires network access"]
async fn test_rtk2go_sourcetable() {
    let config = NtripConfig::new("rtk2go.com", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    assert!(!table.streams.is_empty(), "Sourcetable should have streams");
    assert!(!table.casters.is_empty(), "Sourcetable should have casters");

    // RTK2go should have many streams
    println!("RTK2go has {} streams", table.streams.len());
    assert!(table.streams.len() > 100, "RTK2go should have >100 streams");
}

/// Test fetching sourcetable from EUREF (European caster)
#[tokio::test]
#[ignore = "requires network access"]
async fn test_euref_sourcetable() {
    let config = NtripConfig::new("euref-ip.net", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    assert!(!table.streams.is_empty(), "Sourcetable should have streams");
    println!("EUREF has {} streams", table.streams.len());
}

/// Test fetching sourcetable from Centipede (French open community)
#[tokio::test]
#[ignore = "requires network access"]
async fn test_centipede_sourcetable() {
    let config = NtripConfig::new("caster.centipede.fr", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    assert!(!table.streams.is_empty(), "Sourcetable should have streams");
    println!("Centipede has {} streams", table.streams.len());
}

/// Test nearest mountpoint calculation
#[tokio::test]
#[ignore = "requires network access"]
async fn test_nearest_mountpoint() {
    let config = NtripConfig::new("rtk2go.com", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    // Find nearest to Brisbane, Australia
    let nearest = table.nearest_rtcm_stream(-27.47, 153.02);
    assert!(nearest.is_some(), "Should find a nearest stream");

    let (stream, dist) = nearest.unwrap();
    println!(
        "Nearest to Brisbane: {} at {:.1} km",
        stream.mountpoint, dist
    );
    assert!(stream.is_rtcm(), "Nearest should be an RTCM stream");
}

/// Test NTRIP v1 protocol connection
#[tokio::test]
#[ignore = "requires network access and valid mountpoint"]
async fn test_v1_connection() {
    let config = NtripConfig::new("rtk2go.com", 2101, "Laguna01")
        .with_credentials("test@example.com", "none")
        .with_version(NtripVersion::V1);

    let mut client = NtripClient::new(config).expect("Failed to create client");

    // Try to connect
    if let Ok(()) = client.connect().await {
        // Read some data
        let mut buf = [0u8; 4096];
        let result =
            tokio::time::timeout(Duration::from_secs(5), client.read_chunk(&mut buf)).await;

        match result {
            Ok(Ok(n)) => {
                println!("Received {} bytes via NTRIP v1", n);
                assert!(n > 0, "Should receive some data");
            }
            Ok(Err(e)) => println!("Read error (may be expected): {}", e),
            Err(_) => println!("Timeout (mountpoint may be offline)"),
        }
    }
}

/// Test NTRIP v2 protocol connection
#[tokio::test]
#[ignore = "requires network access and valid mountpoint"]
async fn test_v2_connection() {
    let config = NtripConfig::new("rtk2go.com", 2101, "Laguna01")
        .with_credentials("test@example.com", "none")
        .with_version(NtripVersion::V2);

    let mut client = NtripClient::new(config).expect("Failed to create client");

    if let Ok(()) = client.connect().await {
        let mut buf = [0u8; 4096];
        let result =
            tokio::time::timeout(Duration::from_secs(5), client.read_chunk(&mut buf)).await;

        match result {
            Ok(Ok(n)) => {
                println!("Received {} bytes via NTRIP v2", n);
                assert!(n > 0, "Should receive some data");
            }
            Ok(Err(e)) => println!("Read error (may be expected): {}", e),
            Err(_) => println!("Timeout (mountpoint may be offline)"),
        }
    }
}

/// Test auto-detect protocol version
#[tokio::test]
#[ignore = "requires network access and valid mountpoint"]
async fn test_auto_protocol_detection() {
    let config = NtripConfig::new("rtk2go.com", 2101, "Laguna01")
        .with_credentials("test@example.com", "none")
        .with_version(NtripVersion::Auto);

    let mut client = NtripClient::new(config).expect("Failed to create client");

    if let Ok(()) = client.connect().await {
        println!("Connected with auto-detect");
        assert!(client.is_connected());
    }
}

/// Test TLS connection (AUSCORS uses HTTPS)
#[tokio::test]
#[ignore = "requires network access and AUSCORS credentials"]
async fn test_tls_sourcetable() {
    let config = NtripConfig::new("ntrip.data.gnss.ga.gov.au", 443, "").with_tls();

    // This will fail without credentials, but should at least establish TLS
    let result = NtripClient::get_sourcetable(&config).await;

    // Even without auth, we might get a response (empty sourcetable)
    match result {
        Ok(table) => println!("Got {} streams from AUSCORS", table.streams.len()),
        Err(e) => println!("Expected error without credentials: {}", e),
    }
}

/// Test connection timeout handling
#[tokio::test]
async fn test_connection_timeout() {
    // Use a non-routable IP to trigger timeout
    let config = NtripConfig::new("10.255.255.1", 2101, "TEST").with_timeout(2);

    let mut client = NtripClient::new(config).expect("Failed to create client");

    let start = std::time::Instant::now();
    let result = client.connect().await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Should fail to connect");
    assert!(elapsed.as_secs() >= 1, "Should wait at least 1 second");
    assert!(
        elapsed.as_secs() <= 5,
        "Should not wait more than 5 seconds"
    );

    println!("Connection timeout after {:?}", elapsed);
}

/// Test configuration validation
#[test]
fn test_config_validation() {
    // Empty host should fail
    let config = NtripConfig::new("", 2101, "TEST");
    assert!(config.validate().is_err());

    // Zero port should fail
    let config = NtripConfig::new("example.com", 0, "TEST");
    assert!(config.validate().is_err());

    // Valid config should pass
    let config = NtripConfig::new("example.com", 2101, "TEST");
    assert!(config.validate().is_ok());
}
