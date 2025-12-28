Jon Listen
=================================
[![CI](https://github.com/cspinetta/jon-listen/workflows/CI/badge.svg)](https://github.com/cspinetta/jon-listen/actions)

A high-performance network logging server that receives log messages over TCP or UDP and writes them to files with automatic rotation. Built with Rust and Tokio for async I/O.

## What is Jon Listen?

Jon Listen is a network logging server that:
- **Receives logs** from applications over TCP or UDP
- **Writes logs** to plain text files with automatic rotation
- **Provides metrics** via Prometheus endpoint for monitoring
- **Handles backpressure** with configurable policies (block or discard)
- **Supports high concurrency** with async/await architecture

Perfect for centralized logging, log aggregation, or as a simple log sink for distributed systems.

## Features

- **Protocol Support**: TCP and UDP server modes
- **File Rotation**: Automatic rotation by day or duration with configurable retention
- **Backpressure Handling**: Configurable policies (Block or Discard) when buffers are full
- **Connection Limits**: Configurable maximum concurrent TCP connections (default: 1000)
- **Prometheus Metrics**: Built-in metrics endpoint for monitoring (default port: 9090)
- **Graceful Shutdown**: Clean shutdown on SIGTERM/SIGINT
- **Environment Configuration**: Override settings via environment variables
- **Async Architecture**: Built on Tokio for high-performance async I/O

![alt text](https://upload.wikimedia.org/wikipedia/commons/4/44/Jon_Postel.jpg)

> *[Jon Postel] in 1994, with map of Internet top-level domains.*

## Requirements

- Rust stable (Edition 2021)

```bash
rustup install stable
rustup default stable
```

## Quick Start

1. **Start the server**:

```bash
RUST_LOG=info cargo run
```

The server will start listening on the configured host and port (default: `0.0.0.0:8080`). It will write received messages to log files in the configured directory.

2. **Send logs** (using the example client):

```bash
# UDP
cargo run --example logging_client -- --address 127.0.0.1:8080 --duration 10

# TCP
cargo run --example logging_client -- --address 127.0.0.1:8080 --duration 10 --tcp
```

3. **View metrics** (if metrics are enabled):

```bash
curl http://localhost:9090/metrics
```

4. **Stop the server**: Press Ctrl+C for graceful shutdown.

## Configuration

Configuration is written in [TOML] format. The default configuration is in [`config/default.toml`](config/default.toml).

### Configuration Files

Jon Listen loads configuration in this order (later values override earlier ones):

1. `config/default.toml` (required)
2. `config/{RUN_MODE}.toml` (optional, e.g., `config/development.toml` or `config/production.toml`)
3. `config/local.toml` (optional, for local overrides)
4. Environment variables with `APP_` prefix

Set `RUN_MODE` environment variable to load environment-specific config:

```bash
RUN_MODE=production cargo run
```

### Key Configuration Options

- **Server**: Protocol (TCP/UDP), host, port, max connections
- **File Writer**: Directory, filename, rotation policy, backpressure policy
- **Rotation**: Policy (ByDay/ByDuration), retention count, duration
- **Metrics**: Prometheus metrics port (default: 9090)

### Environment Variables

**Log Level**: Control application logging with `RUST_LOG`:

```bash
RUST_LOG=info cargo run                    # Set global log level
RUST_LOG=writer=debug cargo run            # Set per-module log level
```

**Configuration Overrides**: Override any config value using `APP_` prefix:

```bash
# Override rotation policy
APP_filewriter_rotation_policy=ByDuration cargo run

# Override server port
APP_server_port=9000 cargo run

# Override multiple settings
RUST_LOG=info APP_server_port=9000 APP_filewriter_rotation_policy=ByDay cargo run
```

The environment variable naming follows the TOML structure: `APP_{section}_{key}` or `APP_{section}_{subsection}_{key}`.


## Run tests

Execute from the terminal:

```bash
cargo test
```

For test statistics summary, use `cargo-nextest`:

```bash
cargo install cargo-nextest
cargo nextest run
```

### Test Coverage

Generate a coverage report:

```bash
./scripts/coverage.sh
```

This will:
- Install `cargo-tarpaulin` if not already installed
- Generate an HTML coverage report in `./coverage/tarpaulin-report.html`
- Display a coverage summary in the terminal

Open `./coverage/tarpaulin-report.html` in your browser to view detailed line-by-line coverage.

### Examples

Simple UDP sender (sends N messages):

```bash
cargo run --example send_via_udp -- 127.0.0.1:8080 1000
```

Minimal logging client (UDP or TCP) for a duration:

```bash
cargo run --example logging_client -- --address 127.0.0.1:8080 --duration 10
cargo run --example logging_client -- --address 127.0.0.1:8080 --duration 10 --tcp
```

## License

Apache-2.0

[Rust]:https://www.rust-lang.org/en-US/index.html
[TOML]:https://github.com/toml-lang/toml
[Jon Postel]:https://en.wikipedia.org/wiki/Jon_Postel
