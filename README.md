# ntrip-core

[![Crates.io](https://img.shields.io/crates/v/ntrip-core.svg)](https://crates.io/crates/ntrip-core)
[![Documentation](https://docs.rs/ntrip-core/badge.svg)](https://docs.rs/ntrip-core)
[![CI](https://github.com/Spark-Autonomy/ntrip-core/workflows/CI/badge.svg)](https://github.com/Spark-Autonomy/ntrip-core/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-blue.svg)](https://www.rust-lang.org)

An async NTRIP client library for Rust.

## Features

- **Protocol Support**: NTRIP v1 (ICY) and v2 (HTTP/1.1 chunked)
- **Security**: TLS/HTTPS via rustls (no OpenSSL dependency)
- **Discovery**: Sourcetable retrieval and nearest mountpoint selection
- **Async**: Built on Tokio for efficient async I/O
- **Robust**: Read timeouts, automatic reconnection (configurable), proper error handling
- **Proxy**: HTTP proxy support via CONNECT tunneling

See the [API documentation](https://docs.rs/ntrip-core) for complete usage details.

## Why ntrip-core?

- **No OpenSSL dependency** - TLS via rustls simplifies cross-compilation and deployment
- **Async-native** - Built on Tokio from the ground up, not blocking wrappers
- **Complete protocol support** - Both NTRIP v1 and v2 with automatic version detection
- **Sourcetable discovery** - Parse sourcetables and find nearest mountpoint by coordinates
- **HTTP proxy support** - CONNECT tunneling with optional proxy authentication
- **Cancellation-safe** - Works naturally with `tokio::select!` for timeouts and shutdown
- **GGA position reporting** - Both in-stream and v2 header methods supported
- **Automatic reconnection** - Configurable retry with GGA state preservation

## Quick Start

```rust
use ntrip_core::{NtripClient, NtripConfig};

#[tokio::main]
async fn main() -> Result<(), ntrip_core::Error> {
    let config = NtripConfig::new("caster.example.com", 2101, "MOUNTPOINT")
        .with_credentials("username", "password");

    let mut client = NtripClient::new(config)?;
    client.connect().await?;

    let mut buf = [0u8; 4096];
    loop {
        let n = client.read_chunk(&mut buf).await?;
        println!("Received {} bytes of RTCM data", n);
    }
}
```

## Automatic Reconnection

By default, the client will automatically attempt to reconnect up to 3 times on connection loss or timeout:

```rust
// Default: 3 reconnection attempts with 1 second delay
let config = NtripConfig::new("caster.example.com", 2101, "MOUNT");

// Custom reconnection settings
let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
    .with_reconnect(5, 2000);  // 5 attempts, 2 second delay

// Disable automatic reconnection
let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
    .without_reconnect();
```

When reconnection occurs, the client automatically resends the last GGA position to maintain VRS/nearest-base selection.

## TLS/HTTPS Connections

TLS capability is always compiled in (via `tokio-rustls`). To connect to a TLS-enabled caster:

```rust
let config = NtripConfig::new("secure-caster.example.com", 443, "MOUNT")
    .with_tls()  // Enable TLS for this connection
    .with_credentials("user", "pass");
```

By default, connections use plain TCP. Call `.with_tls()` to enable TLS.

## HTTP Proxy

Connect through an HTTP proxy using the CONNECT method:

```rust
use ntrip_core::{NtripConfig, ProxyConfig};

// Explicit proxy configuration
let proxy = ProxyConfig::new("proxy.example.com", 8080)
    .with_credentials("proxy_user", "proxy_pass");

let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
    .with_proxy(proxy);

// Or read from $HTTP_PROXY environment variable
let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
    .with_proxy_from_env();
```

## Sourcetable Discovery

```rust
use ntrip_core::{NtripClient, NtripConfig};

let config = NtripConfig::new("rtk2go.com", 2101, "");
let table = NtripClient::get_sourcetable(&config).await?;

// Find nearest mountpoint
if let Some((stream, distance_km)) = table.nearest_rtcm_stream(lat, lon) {
    println!("Nearest: {} at {:.1} km", stream.mountpoint, distance_km);
}
```

## Examples

```bash
# Fetch sourcetable from a caster
cargo run --example sourcetable -- rtk2go.com

# Find nearest mountpoint to a location
cargo run --example nearest -- rtk2go.com -27.47 153.02

# Connect and stream RTCM data
cargo run --example connect -- rtk2go.com Laguna01 2101 --user=you@example.com --pass=none
```

## Testing

```bash
# Run unit tests
cargo test

# Run integration tests against real public casters (no credentials needed)
just test-integration
# or: cargo test --test integration -- --ignored

# Run shell-based test suite against real casters
just test-suite          # Sourcetable only
just test-suite-full     # + nearest mountpoint tests
just test-suite-connect  # + connection tests (requires credentials)
```

The integration tests fetch sourcetables from all public casters listed below, and include dynamic connection tests that find active mountpoints and stream real RTCM data from RTK2go and Centipede - no private credentials required.

For the shell-based connection tests, copy `scripts/credentials.env.template` to `scripts/credentials.env` and fill in your credentials.

## Logging

This crate uses [`tracing`](https://docs.rs/tracing) for structured logging. Enable with a subscriber:

```rust
tracing_subscriber::fmt::init();
```

## Tested Casters

This library is regularly tested against these public NTRIP casters:

| Caster | Host | Auth | Notes |
|--------|------|------|-------|
| **RTK2go** | rtk2go.com:2101 | Email/none | Large public caster, 1000+ streams |
| **Centipede** | caster.centipede.fr:2101 | None | Open French/EU RTK network |
| **EUREF** | euref-ip.net:2101 | None* | European reference stations |
| **IGS** | igs-ip.net:2101 | None* | International GNSS Service |
| **SNIP Demo** | ntrip.use-snip.com:2101 | None | Demo caster for testing |
| **AUSCORS** | ntrip.data.gnss.ga.gov.au:443 | Required | Geoscience Australia (HTTPS) |

\* Some streams may require registration

## Development

We use [Just](https://github.com/casey/just) as a command runner:

```bash
just check          # Run all checks (fmt, clippy, test)
just test           # Run tests
just fmt            # Format code
just test-suite     # Run test suite against real NTRIP casters
```

## Minimum Supported Rust Version

MSRV is **1.75**.

## License

Licensed under MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)
