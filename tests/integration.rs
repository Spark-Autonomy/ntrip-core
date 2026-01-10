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

/// Test fetching sourcetable from SNIP Demo Caster
#[tokio::test]
#[ignore = "requires network access"]
async fn test_snip_demo_sourcetable() {
    let config = NtripConfig::new("ntrip.use-snip.com", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    // SNIP demo caster should have streams
    println!("SNIP Demo has {} streams", table.streams.len());
    // At minimum there should be caster info
    assert!(
        !table.streams.is_empty() || !table.casters.is_empty(),
        "SNIP should have streams or caster entries"
    );
}

/// Test fetching sourcetable from IGS (International GNSS Service)
#[tokio::test]
#[ignore = "requires network access"]
async fn test_igs_sourcetable() {
    let config = NtripConfig::new("igs-ip.net", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    assert!(!table.streams.is_empty(), "Sourcetable should have streams");
    println!("IGS has {} streams", table.streams.len());
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

/// Test dynamic connection to RTK2go using discovered mountpoint
///
/// This test fetches the sourcetable, finds a mountpoint near a well-known
/// location, and attempts to connect. No private credentials needed -
/// RTK2go accepts any email as username with "none" as password.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_rtk2go_dynamic_connection() {
    // Step 1: Fetch sourcetable
    let config = NtripConfig::new("rtk2go.com", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    // Step 2: Find nearest RTCM stream to New York (likely to have active stations)
    let nearest = table.nearest_rtcm_stream(40.71, -74.01);
    let mountpoint = match nearest {
        Some((stream, dist)) => {
            println!(
                "Found mountpoint '{}' at {:.1} km from New York",
                stream.mountpoint, dist
            );
            stream.mountpoint.clone()
        }
        None => {
            // Fallback: just pick the first RTCM stream
            let first_rtcm = table.streams.iter().find(|s| s.is_rtcm());
            match first_rtcm {
                Some(stream) => {
                    println!("Using first RTCM stream: {}", stream.mountpoint);
                    stream.mountpoint.clone()
                }
                None => {
                    panic!("No RTCM streams found in RTK2go sourcetable");
                }
            }
        }
    };

    // Step 3: Connect to the discovered mountpoint
    // RTK2go uses email as username, "none" as password (publicly documented)
    let config = NtripConfig::new("rtk2go.com", 2101, &mountpoint)
        .with_credentials("ntrip-core-test@example.com", "none")
        .with_timeout(10)
        .without_reconnect();

    let mut client = NtripClient::new(config).expect("Failed to create client");

    match client.connect().await {
        Ok(()) => {
            println!("Successfully connected to {}", mountpoint);
            assert!(client.is_connected());

            // Try to read some data
            let mut buf = [0u8; 4096];
            let result =
                tokio::time::timeout(Duration::from_secs(5), client.read_chunk(&mut buf)).await;

            match result {
                Ok(Ok(n)) => {
                    println!("Received {} bytes of RTCM data from {}", n, mountpoint);
                    assert!(n > 0, "Should receive some data");
                }
                Ok(Err(e)) => {
                    // Stream might be offline even if connection succeeded
                    println!("Read error (mountpoint may be inactive): {}", e);
                }
                Err(_) => {
                    println!("Timeout waiting for data (mountpoint may be inactive)");
                }
            }
        }
        Err(e) => {
            // Connection failure is acceptable for this test - mountpoint may be offline
            println!(
                "Could not connect to {} (may be offline): {}",
                mountpoint, e
            );
        }
    }
}

/// Test dynamic connection to Centipede (French open RTK network)
///
/// Centipede is a fully open community network - no credentials required.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_centipede_dynamic_connection() {
    // Step 1: Fetch sourcetable
    let config = NtripConfig::new("caster.centipede.fr", 2101, "");
    let table = NtripClient::get_sourcetable(&config)
        .await
        .expect("Failed to fetch sourcetable");

    // Step 2: Find nearest RTCM stream to Paris
    let nearest = table.nearest_rtcm_stream(48.86, 2.35);
    let mountpoint = match nearest {
        Some((stream, dist)) => {
            println!(
                "Found Centipede mountpoint '{}' at {:.1} km from Paris",
                stream.mountpoint, dist
            );
            stream.mountpoint.clone()
        }
        None => {
            panic!("No RTCM streams found in Centipede sourcetable");
        }
    };

    // Step 3: Connect - Centipede is fully open (no credentials)
    let config = NtripConfig::new("caster.centipede.fr", 2101, &mountpoint)
        .with_timeout(10)
        .without_reconnect();

    let mut client = NtripClient::new(config).expect("Failed to create client");

    match client.connect().await {
        Ok(()) => {
            println!("Successfully connected to Centipede/{}", mountpoint);
            assert!(client.is_connected());

            let mut buf = [0u8; 4096];
            let result =
                tokio::time::timeout(Duration::from_secs(5), client.read_chunk(&mut buf)).await;

            match result {
                Ok(Ok(n)) => {
                    println!("Received {} bytes from Centipede/{}", n, mountpoint);
                    assert!(n > 0);
                }
                Ok(Err(e)) => println!("Read error: {}", e),
                Err(_) => println!("Timeout (mountpoint may be inactive)"),
            }
        }
        Err(e) => {
            println!("Could not connect to Centipede/{}: {}", mountpoint, e);
        }
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

/// Test connection failure handling.
///
/// Note: This test uses a non-routable IP address which may fail immediately
/// with ICMP unreachable or timeout depending on network configuration.
/// The test verifies that the error is handled correctly regardless of timing.
#[tokio::test]
async fn test_connection_failure() {
    // Use a non-routable IP - connection should fail (either timeout or immediate rejection)
    let config = NtripConfig::new("10.255.255.1", 2101, "TEST")
        .with_timeout(2)
        .without_reconnect();

    let mut client = NtripClient::new(config).expect("Failed to create client");

    let result = client.connect().await;

    // Should fail - either with timeout or connection refused depending on network
    assert!(result.is_err(), "Should fail to connect");

    let err = result.unwrap_err();
    let err_str = format!("{}", err);

    // Error should be either a timeout or connection failure
    assert!(
        err_str.contains("timed out")
            || err_str.contains("Failed to connect")
            || err_str.contains("Connection refused")
            || err_str.contains("Network is unreachable"),
        "Unexpected error: {}",
        err
    );

    println!("Connection failed as expected: {}", err);
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
