# Contributing to ntrip-core

Thank you for your interest in contributing to ntrip-core!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ntrip-core.git`
3. Create a branch: `git checkout -b my-feature`

## Development Setup

We use [Just](https://github.com/casey/just) as a command runner. Install it and run:

```bash
just check    # Run all checks (fmt, clippy, test)
just test     # Run tests only
just fmt      # Format code
just clippy   # Run lints
```

## Making Changes

1. **Write tests** for new functionality
2. **Run `just check`** before committing
3. **Update documentation** if you change public APIs
4. **Follow Rust conventions** and existing code style

## Pull Request Process

1. Ensure CI passes
2. Update CHANGELOG.md with your changes
3. Request a review from maintainers

## Testing Against Real Casters

Integration tests are ignored by default. To run them:

```bash
cargo test --test integration -- --ignored
```

**Note:** These require network access and may fail if casters are unavailable.

## Code of Conduct

Be respectful and constructive. We're all here to build great software.

## Questions?

Open an issue or discussion on GitHub.
