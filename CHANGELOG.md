# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/greenforge-labs/ntrip-core/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/greenforge-labs/ntrip-core/releases/tag/v0.1.0
