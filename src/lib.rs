//! # ntrip-core
//!
//! An async NTRIP client library for Rust.
//!
//! [NTRIP](https://en.wikipedia.org/wiki/Networked_Transport_of_RTCM_via_Internet_Protocol)
//! (Networked Transport of RTCM via Internet Protocol) is a protocol for
//! streaming GNSS correction data over the internet. This library provides
//! a client implementation for receiving RTK corrections from NTRIP casters.
//!
//! ## Features
//!
//! - **Protocol Support**: NTRIP v1 (ICY) and v2 (HTTP/1.1 chunked)
//! - **Security**: TLS/HTTPS via rustls (no OpenSSL dependency)
//! - **Discovery**: Sourcetable retrieval and nearest mountpoint selection
//! - **Proxy**: HTTP proxy support via CONNECT tunneling
//! - **Async**: Built on Tokio for efficient async I/O
//! - **Robust**: Read timeouts, automatic reconnection, and proper error handling
//!
//! ## Reconnection Behavior
//!
//! By default, [`NtripClient::read_chunk()`] will automatically attempt to reconnect
//! on timeout or disconnect (up to 3 attempts with 1 second delay). This means the
//! method may block for longer than the read timeout while reconnecting.
//!
//! To disable automatic reconnection, use [`NtripConfig::without_reconnect()`].
//!
//! ## TLS Support
//!
//! TLS/HTTPS capability is always compiled in via `tokio-rustls` (no OpenSSL dependency).
//! TLS is **not** used by default - you must explicitly enable it per-connection:
//!
//! ```rust,no_run
//! # use ntrip_core::NtripConfig;
//! let config = NtripConfig::new("secure-caster.example.com", 443, "MOUNT")
//!     .with_tls();  // Enable TLS for this connection
//! ```
//!
//! ## HTTP Proxy Support
//!
//! Connect through an HTTP proxy using the CONNECT method:
//!
//! ```rust,no_run
//! use ntrip_core::{NtripConfig, ProxyConfig};
//!
//! // Explicit proxy configuration
//! let proxy = ProxyConfig::new("proxy.example.com", 8080)
//!     .with_credentials("proxy_user", "proxy_pass");
//!
//! let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
//!     .with_proxy(proxy);
//!
//! // Or read from $HTTP_PROXY environment variable
//! let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
//!     .with_proxy_from_env();
//! ```
//!
//! ## Cancellation
//!
//! To cancel a pending operation, use `tokio::select!`:
//!
//! ```rust,no_run
//! use ntrip_core::{NtripClient, NtripConfig};
//! use tokio::sync::oneshot;
//!
//! # async fn example() -> Result<(), ntrip_core::Error> {
//! let config = NtripConfig::new("rtk2go.com", 2101, "MOUNT");
//! let mut client = NtripClient::new(config)?;
//! client.connect().await?;
//!
//! let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
//!
//! let mut buf = [0u8; 4096];
//! tokio::select! {
//!     result = client.read_chunk(&mut buf) => {
//!         match result {
//!             Ok(n) => println!("Received {} bytes", n),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//!     _ = cancel_rx => {
//!         println!("Operation cancelled");
//!         client.disconnect();
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Timeout Handling
//!
//! For fine-grained timeout control beyond the built-in read timeout:
//!
//! ```rust,no_run
//! use ntrip_core::{NtripClient, NtripConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), ntrip_core::Error> {
//! // Disable built-in reconnection for manual control
//! let config = NtripConfig::new("rtk2go.com", 2101, "MOUNT")
//!     .with_read_timeout(0)  // Disable built-in read timeout
//!     .without_reconnect();  // Disable auto-reconnection
//!
//! let mut client = NtripClient::new(config)?;
//! client.connect().await?;
//!
//! let mut buf = [0u8; 4096];
//!
//! // Apply your own timeout
//! match tokio::time::timeout(Duration::from_secs(5), client.read_chunk(&mut buf)).await {
//!     Ok(Ok(n)) => println!("Received {} bytes", n),
//!     Ok(Err(e)) => eprintln!("Read error: {}", e),
//!     Err(_) => eprintln!("Custom timeout expired"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ntrip_core::{NtripClient, NtripConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ntrip_core::Error> {
//!     let config = NtripConfig::new("rtk2go.com", 2101, "MOUNTPOINT")
//!         .with_credentials("user@example.com", "none");
//!
//!     let mut client = NtripClient::new(config)?;
//!     client.connect().await?;
//!
//!     let mut buf = [0u8; 4096];
//!     let n = client.read_chunk(&mut buf).await?;
//!     println!("Received {} bytes of RTCM data", n);
//!     Ok(())
//! }
//! ```
//!
//! ## Sourcetable Discovery
//!
//! ```rust,no_run
//! use ntrip_core::{NtripClient, NtripConfig};
//!
//! # async fn example() -> Result<(), ntrip_core::Error> {
//! let config = NtripConfig::new("rtk2go.com", 2101, "");
//! let table = NtripClient::get_sourcetable(&config).await?;
//!
//! // Find nearest mountpoint
//! if let Some((stream, distance_km)) = table.nearest_rtcm_stream(-27.47, 153.02) {
//!     println!("Nearest: {} at {:.1} km", stream.mountpoint, distance_km);
//! }
//! # Ok(())
//! # }
//! ```

mod client;
mod config;
mod error;
mod gga;
mod sourcetable;
mod stream;

// Re-export public API
pub use client::NtripClient;
pub use config::{ConnectionConfig, NtripConfig, NtripVersion, ProxyConfig};
pub use error::Error;
pub use gga::GgaSentence;
pub use sourcetable::{CasterEntry, NetworkEntry, Sourcetable, StreamEntry};
