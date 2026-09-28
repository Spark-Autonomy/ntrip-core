# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-01-10

### Added

- HTTP proxy support via CONNECT tunneling
- Integration tests against SNIP Demo Caster and IGS
- Dynamic connection tests for RTK2go and Centipede (no credentials required)
- `ProxyConfig` struct for proxy configuration
- `NtripConfig::with_proxy()` for explicit proxy settings
- `NtripConfig::with_proxy_from_env()` to read from `$HTTP_PROXY` environment variable
- `ProxyConfig::from_url()` for parsing proxy URLs
- Proxy authentication support (Basic auth)

### Changed

- Improved README with feature highlights and usage examples
- Added documentation for logging via `tracing`
- Expanded tested casters documentation in README

## [0.1.0] - 2025-12-20

### Added

- Initial release of ntrip-core
- NTRIP v1 (ICY) protocol support
- NTRIP v2 (HTTP/1.1 chunked) protocol support
- Auto-detection of protocol version from server response
- TLS/HTTPS support via rustls (no OpenSSL dependency)
- Basic authentication support
- Sourcetable retrieval and parsing
- Nearest mountpoint selection (Haversine distance)
- GGA position reporting with `Ntrip-GGA` header support for v2
- Read timeouts with configurable duration
- Automatic reconnection on disconnect/timeout (configurable)
- Comprehensive error handling
- Examples: connect, sourcetable, nearest
- Integration tests against public casters
- GitHub Actions CI workflow

[Unreleased]: https://github.com/Spark-Autonomy/ntrip-core/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Spark-Autonomy/ntrip-core/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Spark-Autonomy/ntrip-core/releases/tag/v0.1.0
