//! Protocol parsing tests using mock TCP servers.
//!
//! These tests validate the client's protocol handling without network access.

use ntrip_core::{NtripClient, NtripConfig};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Test that header over-read bytes are preserved (P0-2 regression test).
///
/// Simulates a server that sends headers and body data in a single TCP packet.
#[tokio::test]
async fn test_header_overread_preserved() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Read the request
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        // Send headers + body data in a single write (simulating a single TCP packet)
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\r\n\xD3\x00\x13RTCM_DATA_HERE";
        socket.write_all(response).await.unwrap();
        let _ = socket.flush().await;

        // Keep connection open long enough for client to read
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let config = NtripConfig::new(addr.ip().to_string(), addr.port(), "TEST")
        .with_timeout(5)
        .with_read_timeout(5)
        .without_reconnect();

    let mut client = NtripClient::new(config).unwrap();
    client.connect().await.unwrap();

    let mut buf = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(1), client.read_chunk(&mut buf))
        .await
        .expect("read timeout")
        .expect("read error");

    // Should receive the RTCM data that was in the same packet as headers
    assert!(n > 0, "Should receive over-read bytes");
    assert_eq!(buf[0], 0xD3, "First byte should be RTCM preamble");

    server.abort();
}

/// Test chunked transfer encoding parsing.
#[tokio::test]
async fn test_chunked_encoding() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Read the request
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        // Send chunked response with all data at once
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nNtrip-Version: Ntrip/2.0\r\n\r\n5\r\nHELLO\r\n5\r\nWORLD\r\n";
        socket.write_all(response).await.unwrap();
        let _ = socket.flush().await;

        // Keep connection open
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let config = NtripConfig::new(addr.ip().to_string(), addr.port(), "TEST")
        .with_timeout(5)
        .with_read_timeout(5)
        .without_reconnect();

    let mut client = NtripClient::new(config).unwrap();
    client.connect().await.unwrap();

    // Read first chunk
    let mut buf = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(1), client.read_chunk(&mut buf))
        .await
        .expect("read timeout")
        .expect("read error");

    assert_eq!(n, 5, "First chunk should be 5 bytes");
    assert_eq!(&buf[..5], b"HELLO");

    // Read second chunk
    let n = tokio::time::timeout(Duration::from_secs(1), client.read_chunk(&mut buf))
        .await
        .expect("read timeout")
        .expect("read error");

    assert_eq!(n, 5, "Second chunk should be 5 bytes");
    assert_eq!(&buf[..5], b"WORLD");

    server.abort();
}

/// Test chunk extension tolerance (P1-3).
#[tokio::test]
async fn test_chunk_extension_tolerance() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        // Send chunked response with chunk extensions - all in one write
        let response =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\na;name=value\r\n0123456789\r\n";
        socket.write_all(response).await.unwrap();
        let _ = socket.flush().await;

        // Keep connection open
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let config = NtripConfig::new(addr.ip().to_string(), addr.port(), "TEST")
        .with_timeout(5)
        .with_read_timeout(5)
        .without_reconnect();

    let mut client = NtripClient::new(config).unwrap();
    client.connect().await.unwrap();

    let mut buf = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(1), client.read_chunk(&mut buf))
        .await
        .expect("read timeout")
        .expect("read error");

    assert_eq!(n, 10, "Should parse chunk size ignoring extension");
    assert_eq!(&buf[..10], b"0123456789");

    server.abort();
}

/// Test status code parsing.
#[tokio::test]
async fn test_status_code_parsing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        // Send 401 Unauthorized
        let response = b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic\r\n\r\n";
        socket.write_all(response).await.unwrap();
    });

    let config = NtripConfig::new(addr.ip().to_string(), addr.port(), "TEST")
        .with_timeout(5)
        .without_reconnect();

    let mut client = NtripClient::new(config).unwrap();
    let result = client.connect().await;

    assert!(result.is_err(), "Should fail on 401");
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("Authentication"),
        "Error should mention authentication"
    );

    server.await.unwrap();
}

/// Test ICY 200 OK response (NTRIP v1).
#[tokio::test]
async fn test_icy_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        // Send ICY response with immediate data
        socket
            .write_all(b"ICY 200 OK\r\n\xD3\x00\x10")
            .await
            .unwrap();
        let _ = socket.flush().await;

        // Keep connection open
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let config = NtripConfig::new(addr.ip().to_string(), addr.port(), "TEST")
        .with_timeout(5)
        .with_read_timeout(5)
        .without_reconnect();

    let mut client = NtripClient::new(config).unwrap();
    client.connect().await.unwrap();

    let mut buf = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_secs(1), client.read_chunk(&mut buf))
        .await
        .expect("read timeout")
        .expect("read error");

    // Should receive the RTCM data after ICY header
    assert!(n >= 3, "Should receive over-read bytes after ICY");
    assert_eq!(buf[0], 0xD3, "First byte should be RTCM preamble");

    server.abort();
}

/// Test read_timeout_secs = 0 means no timeout (P0-1 regression test).
#[tokio::test]
async fn test_zero_read_timeout_no_immediate_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();

        // Send successful response
        socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();

        // Wait a bit before sending data (longer than a 0-second timeout would allow)
        tokio::time::sleep(Duration::from_millis(100)).await;
        socket.write_all(b"DATA").await.unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
    });

    let config = NtripConfig::new(addr.ip().to_string(), addr.port(), "TEST")
        .with_timeout(5)
        .with_read_timeout(0) // No timeout
        .without_reconnect();

    let mut client = NtripClient::new(config).unwrap();
    client.connect().await.unwrap();

    // With read_timeout_secs = 0, this should NOT immediately fail
    // We add our own timeout to avoid hanging forever
    let mut buf = [0u8; 256];
    let result = tokio::time::timeout(Duration::from_secs(1), client.read_chunk(&mut buf)).await;

    // The read should succeed (data arrives after 100ms)
    assert!(
        result.is_ok(),
        "Should not immediately timeout with read_timeout_secs=0"
    );
    let n = result.unwrap().unwrap();
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"DATA");

    server.await.unwrap();
}

/// Test header injection validation (P1-4).
#[test]
fn test_header_injection_validation() {
    // Mountpoint with newline should fail
    let config = NtripConfig::new("example.com", 2101, "TEST\r\nX-Injected: value");
    assert!(
        config.validate().is_err(),
        "Mountpoint with CRLF should fail validation"
    );

    // Username with control characters should fail
    let config =
        NtripConfig::new("example.com", 2101, "TEST").with_credentials("user\nname", "pass");
    assert!(
        config.validate().is_err(),
        "Username with newline should fail validation"
    );

    // Host with control characters should fail
    let config = NtripConfig::new("example\r.com", 2101, "TEST");
    assert!(
        config.validate().is_err(),
        "Host with CR should fail validation"
    );

    // Valid config should pass
    let config =
        NtripConfig::new("example.com", 2101, "TEST").with_credentials("user@test.com", "pass123");
    assert!(config.validate().is_ok(), "Valid config should pass");
}

/// Test password redaction in Debug output (P2-1).
#[test]
fn test_password_redacted_in_debug() {
    let config =
        NtripConfig::new("example.com", 2101, "TEST").with_credentials("user", "secret_password");

    let debug_output = format!("{:?}", config);

    assert!(
        !debug_output.contains("secret_password"),
        "Debug output should not contain password"
    );
    assert!(
        debug_output.contains("[REDACTED]"),
        "Debug output should show [REDACTED]"
    );
    assert!(
        debug_output.contains("user"),
        "Debug output should still show username"
    );
}
