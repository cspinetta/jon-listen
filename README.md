Jon Listen
=================================
[![Build Status](https://travis-ci.org/cspinetta/jon-listen.svg?branch=master)](https://travis-ci.org/cspinetta/jon-listen)

Simple and multithreading TCP/UDP logger. It writes data to a plain text file and it also provides additional functionalities such as log file rotation, control the number of rotated files, etc.

Written in [Rust] language.

![alt text](https://upload.wikimedia.org/wikipedia/commons/4/44/Jon_Postel.jpg)

> *[Jon Postel] in 1994, with map of Internet top-level domains.*

## Requirements

- Rust stable (Edition 2021)

```bash
rustup install stable
rustup default stable
```

## Start server

Run from the terminal:

```bash
RUST_LOG=info cargo run
```

- Press Ctrl+C to gracefully stop the server.

## Config file
The config is written in [TOML].
Default config is set in [./config/default.toml](https://github.com/cspinetta/jon-listen/blob/master/config/default.toml).
Optionally you can add a config file by environment setting `RUN_MODE={development|production|anything}` in your environment and providing the appropriate file: `./config/{development|production|anything}.toml`

### Config from the environment

You can provide environment variable to define log level and override configuration:

* Log level: `RUST_LOG={debug|info|warn|error}`. You can set per-module levels: `RUST_LOG=writer=debug` enables debug for the `writer` module.
* Override config: define variables with a prefix of `APP_`. Eg:

`APP_filewriter_rotation_policy=ByDay` would set:

```toml
[filewriter.rotation]
policy = "ByDay"
```

*Running with inline environment variable from the terminal:*

```bash
RUST_LOG=info APP_filewriter_rotation_policy=ByDuration cargo run
```


## Run tests

Execute from the terminal:

```bash
cargo test
```

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
