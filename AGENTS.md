# AGENTS.md

Guidelines for AI coding assistants contributing to Jon Listen.

## Project Philosophy

Jon Listen is a **minimal, high-performance network log collector** - not a logging platform. It does one thing well: accept log events over the network and persist them safely to disk.

**Core principles:**
- Stay small, auditable, and predictable
- Keep code minimalistic and maintainable
- Avoid unnecessary complexity or features outside the core scope
- Prefer explicit behavior over implicit magic

## Development Environment

### Setup
- Rust Edition: 2021
- Minimum Rust version: stable (check `.github/workflows/ci.yml` for exact versions)
- Install Rust: `rustup install stable && rustup default stable`

### Build Commands
```bash
# Development build
cargo build

# Release build (optimized for production)
cargo build --release

# Static binary (Linux)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

### Running the Application
```bash
# Development mode
RUST_LOG=info cargo run

# With TCP protocol
APP_server_protocol=TCP RUST_LOG=info cargo run

# Release binary
RUST_LOG=info ./target/release/jon-listen
```

## Code Style and Conventions

### Logging
- **Always use log macros** (`log::info!`, `log::warn!`, `log::error!`, `log::debug!`)
- **Never use `eprintln!` or `println!`** in source code (test files are acceptable)
- Logging writes to stderr via `pretty_env_logger` (avoids feedback loops with FileWriter)
- Use appropriate log levels:
  - `error!` - Errors that need attention
  - `warn!` - Warnings about potential issues
  - `info!` - Important operational events
  - `debug!` - Detailed debugging information

### Comments
- Keep comments **meaningful and helpful**
- Remove unnecessary clarifications that don't add value
- Add module-level comments explaining what was intentionally left out of tests and why
- Avoid obvious comments that just restate the code

### Error Handling
- Use `anyhow::Result` for application-level errors
- Use `thiserror` for custom error types that need structured error handling
- Always provide context with `.context()` or `?` operator
- Include descriptive error messages in assertions

### Async/Await
- Use Tokio async/await throughout (no hybrid threading model)
- Prefer `tokio::spawn` for concurrent tasks
- Use `tokio::select!` for handling multiple async operations
- Use `Arc` for shared state across async tasks

### Code Organization
- Keep functions focused and single-purpose
- Prefer explicit behavior over implicit
- Use descriptive variable and function names
- Group related functionality in modules (`src/listener/`, `src/writer/`)

## Testing Guidelines

### Test Structure
- Tests live in `tests/` directory with `_spec.rs` suffix
- Use `#[tokio::test]` for async tests
- Use `#[test]` for synchronous tests
- Share test helpers via `tests/helpers/mod.rs`

### Test Principles
- **Remove empty tests** with `assert!(true)` - they create false expectations
- Add module-level comments explaining what scenarios were intentionally not tested and why
- Keep tests **clean and minimalistic**
- Use descriptive assertion messages that include actual values when assertions fail
- Example: `assert!(condition, "Expected X, got: {}", actual_value)`

### Test Execution
```bash
# Run all tests
cargo test

# Run with nextest (better for parallel execution)
cargo install cargo-nextest
cargo nextest run

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Test Coverage
- Generate coverage reports: `./scripts/coverage.sh`
- Coverage reports are in `./coverage/tarpaulin-report.html`
- Exclude test files and examples from coverage calculations

### Parallel Test Considerations
- Metrics initialization (`metrics::init()`) is idempotent - safe to call multiple times
- Tests may share global metrics state when running in parallel
- Ensure tests don't depend on specific metric values when other tests might interfere
- Use unique port numbers for metrics tests to avoid conflicts

## Code Quality Checks

### Before Committing
Always run these checks locally:

```bash
# Format code
cargo fmt --all

# Lint with clippy (must pass with -D warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Build
cargo build --all-features

# Run tests
cargo test --all-features
```

### CI Pipeline
The CI pipeline (`.github/workflows/ci.yml`) runs:
1. Format check (`cargo fmt --all -- --check`)
2. Clippy linting (`cargo clippy --all-targets --all-features -- -D warnings`)
3. Build (`cargo build --verbose --all-features`)
4. Tests (`cargo test --verbose --all-features`)
5. Coverage generation (separate job)

**All checks must pass before merging.**

## Configuration

### Configuration Files
- Default config: `config/default.toml` (required)
- Environment-specific: `config/{RUN_MODE}.toml` (optional, defaults to `development`)
- Local overrides: `config/local.toml` (optional, gitignored)
- Environment variables: `APP_{section}_{key}` format

### Configuration Loading Order
1. `config/default.toml`
2. `config/{RUN_MODE}.toml` (if `RUN_MODE` env var is set, defaults to `development`)
3. `config/local.toml`
4. Environment variables with `APP_` prefix

## Common Patterns

### Metrics
- Use `metrics::counter!`, `metrics::gauge!`, `metrics::histogram!` macros
- Metrics are registered automatically on first use
- Access handle via `metrics::get_handle()` for rendering Prometheus output

### File Operations
- Use `tokio::fs` for async file operations
- File rotation policies: `ByDay` or `ByDuration`
- Backpressure policies: `Block` or `Discard`

### Network Operations
- TCP: Uses `LinesCodec` for newline-delimited messages
- UDP: One event per datagram
- Use `tokio::net::TcpListener` and `tokio::net::UdpSocket`

## PR and Commit Guidelines

### Commit Messages
- Use conventional commit format when appropriate
- Be concise and clear
- Focus on the main change first, then supporting changes
- Example: `refactor: migrate to Tokio async/await`

### Pull Requests
- Ensure all CI checks pass
- Update documentation if behavior changes
- Add tests for new functionality
- Keep PRs focused and reviewable
- Reference related issues if applicable

## What NOT to Do

- Don't add features outside the core scope (parsing, routing, transformation)
- Don't use `eprintln!` or `println!` in source code
- Don't create empty tests with `assert!(true)`
- Don't add unnecessary comments
- Don't ignore clippy warnings
- Don't commit without running tests locally
- Don't add dependencies without justification
- Don't break the minimal, focused design philosophy

## Additional Resources

- `README.md` - Project overview and user documentation
- `TESTING.md` - Guide for local server testing
- `.github/workflows/ci.yml` - CI pipeline configuration
- `Cargo.toml` - Dependencies and build configuration
