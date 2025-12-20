# ntrip-core

An async NTRIP client library for Rust.

## Features

- **Protocol Support**: NTRIP v1 (ICY) and v2 (HTTP/1.1 chunked)
- **Security**: TLS/HTTPS via rustls (no OpenSSL dependency)
- **Discovery**: Sourcetable retrieval and nearest mountpoint selection
- **Async**: Built on Tokio for efficient async I/O
- **Robust**: Read timeouts, automatic reconnection (configurable), proper error handling

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

# Run comprehensive test suite against real casters
bash ./scripts/test_suite.sh --quick   # Sourcetable only
bash ./scripts/test_suite.sh --full    # + nearest mountpoint tests
bash ./scripts/test_suite.sh --connect # + connection tests (requires credentials)
```

For connection tests, copy `scripts/credentials.env.template` to `scripts/credentials.env` and fill in your credentials.

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
