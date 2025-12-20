# ntrip-core development commands

# Default recipe - show available commands
default:
    @just --list

# Run all checks (fmt, clippy, test)
check: fmt-check clippy test
    @echo "All checks passed!"

# Run tests
test:
    cargo test --all-features

# Run tests with output
test-verbose:
    cargo test --all-features -- --nocapture

# Run clippy lints
clippy:
    cargo clippy --all-features -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Build the library
build:
    cargo build --all-features

# Build in release mode
build-release:
    cargo build --release --all-features

# Generate documentation
doc:
    cargo doc --no-deps --all-features --open

# Clean build artifacts
clean:
    cargo clean

# Run example: connect
example-connect:
    cargo run --example connect

# Run example: sourcetable
example-sourcetable:
    cargo run --example sourcetable

# Run integration tests against real casters
test-integration:
    cargo test --test integration -- --ignored

# Prepare for release (all checks + docs)
release-check: check
    cargo doc --no-deps --all-features
    @echo "Ready for release!"

# Publish to crates.io (dry-run)
publish-dry:
    cargo publish --dry-run

# Publish to crates.io
publish:
    cargo publish

# Run test suite (quick - sourcetable only)
test-suite:
    bash ./scripts/test_suite.sh --quick

# Run test suite (full - includes nearest)
test-suite-full:
    bash ./scripts/test_suite.sh --full

# Run test suite (with connections - requires credentials)
test-suite-connect:
    bash ./scripts/test_suite.sh --connect

# Run all tests including stress tests
test-suite-all:
    bash ./scripts/test_suite.sh --all
