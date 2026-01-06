Jon Listen
=================================
[![CI](https://github.com/cspinetta/jon-listen/workflows/CI/badge.svg)](https://github.com/cspinetta/jon-listen/actions)

A **minimal, high-performance network log collector** that receives log messages over TCP or UDP and writes them to rotating files.

Built in **Rust** on **Tokio** for predictable performance, bounded resource usage, and operational simplicity.

![alt text](https://upload.wikimedia.org/wikipedia/commons/4/44/Jon_Postel.jpg)

> *[Jon Postel] in 1994, with map of Internet top-level domains.*

## What it does (and what it doesn’t)

**Jon Listen is a log _collector_, not a logging platform.**

It does one thing well:

> **Accept log events over the network and persist them safely to disk.**

### What it does
- Listens on **TCP or UDP**
- Accepts **arbitrary text**
- Uses **explicit event framing**
- Writes to **plain text files**
- Rotates files deterministically
- Applies **explicit backpressure policies**
- Exposes **Prometheus metrics**
- Stays small, auditable, and predictable

### What it does NOT do
- Parse syslog (RFC3164 / RFC5424)
- Parse or validate JSON
- Enrich, transform, or filter messages
- Route logs to multiple destinations
- Act as a logging pipeline or platform
- Replace Fluent Bit, Vector, or Logstash

If you need parsing, enrichment, routing, or multiple sinks, a full logging **platform** is a better fit.

## Event framing semantics

Event boundaries are explicit and intentional:

- **TCP**
  - Stream-based
  - **Newline (`\n`) delimited**
  - Multiple events per connection
- **UDP**
  - Datagram-based
  - **One event per datagram**
  - A trailing newline is appended if missing

Jon Listen does **not** interpret message contents.

## Typical use cases

Jon Listen is a good fit when you want:

- A **single binary** with no plugin ecosystem
- A **drop-in network log sink**
- No syslog semantics or formatting requirements
- Explicit control over **file rotation and backpressure**
- Logs written **directly to disk**
- A tool you can understand in minutes, not days

Common scenarios:
- Centralized log capture for internal services
- Edge, appliance, or air-gapped environments
- Incident capture / temporary aggregation
- Replacing ad-hoc `nc | logrotate`-style setups
- Existing applications with custom TCP/UDP appenders

## Comparison with other log solutions

| Tool | Category | Message format | Typical role |
|-----|--------|----------------|--------------|
| **Jon Listen** | Collector | Arbitrary text | Simple network sink |
| **syslog (rsyslog / syslog-ng)** | Collector / Router | Syslog format | OS-level logging |
| **Vector** | Platform | Structured / parsed | Observability pipeline |
| **Fluent Bit** | Platform | Structured / parsed | Log forwarding agent |

### Key differences

**Jon Listen vs syslog**
- Accepts **any text**
- No syslog framing, PRI, facilities, or severities
- Lower cognitive overhead for application developers
- Explicit file rotation and backpressure semantics

**Jon Listen vs Vector / Fluent Bit**
- Not a pipeline
- No parsing, routing, or plugins
- Smaller footprint and simpler configuration
- Designed as a sink, not an agent or platform

## Features

- **Protocols**: TCP and UDP
- **Event framing**: newline-delimited (TCP), datagram-delimited (UDP)
- **File rotation**: by day or duration, configurable retention
- **Backpressure**: Block or Discard policies
- **Connection limits**: configurable max concurrent TCP connections
- **Metrics**: Prometheus endpoint
- **Graceful shutdown**: SIGTERM / SIGINT handling
- **Async I/O**: Tokio-based architecture

## Near-term improvements

The following items are being considered to further harden Jon Listen **as a log collector**.
This is not a commitment or a timeline, but a statement of direction.

| Feature | Description | Scope |
|-------|------------|-------|
| **TLS (TCP)** | Encrypt log transport over TCP | Security |
| **mTLS (TCP)** | Mutual authentication using client certificates | Security |
| **Connection rate limiting** | Limit new connections per time period (currently only max concurrent connections is implemented) | Safety |
| **Max line size enforcement** | Prevent unbounded memory usage from oversized messages | Safety |
| **Metrics hardening** | Configurable bind address, clearer failure metrics | Operability |
| **SIGHUP reload (optional)** | Reload config or reopen files without restart | Operability |

## Requirements

- Rust stable (Edition 2021)

```bash
rustup install stable
rustup default stable
```

## Building the binary

Jon Listen is designed to run as a **single standalone binary**.

### Build a release binary

```bash
cargo build --release
```

The binary will be available at:

```bash
target/release/jon-listen
```

You can copy this binary to another machine and run it directly.

## Run the binary

```bash
RUST_LOG=info ./target/release/jon-listen
```

By default, Jon Listen loads configuration from the `config/` directory relative to the working directory.

### Optional: static binary (Linux)

To build a static binary using musl:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

The resulting binary:

```bash
target/x86_64-unknown-linux-musl/release/jon-listen
```

This is useful for minimal containers or systems without a full libc.

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
2. `config/{RUN_MODE}.toml` (optional, defaults to `config/development.toml` if `RUN_MODE` is not set)
3. `config/local.toml` (optional, for local overrides)
4. Environment variables with `APP_` prefix

Set `RUN_MODE` environment variable to load environment-specific config:

```bash
RUN_MODE=production cargo run
```

If `RUN_MODE` is not set, it defaults to `development` and will attempt to load `config/development.toml` (if it exists).

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
