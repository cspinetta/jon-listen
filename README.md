Jon Listen
=================================
[![Build Status](https://travis-ci.org/cspinetta/jon-listen.svg?branch=master)](https://travis-ci.org/cspinetta/jon-listen)

Simple and multithreading UDP listener to forward data to a log file. Also it provide some functionality of a logger such as file rotation, control the number of rotated files, etc.

Written in [Rust] language.

## Dependencies

- Rust nightly:

```bash
rustup install nightly
rustup default nightly

```

## Start server

Execute from the terminal:

```bash
RUST_LOG=info cargo run
```

## Config file
The config is written in [TOML].
Default config is set in [./config/default.toml](https://github.com/cspinetta/jon-listen/blob/master/config/default.toml).
Optionally you can add a config file by environment setting `RUN_MODE={development|production|anything}` in your environment and providing the appropriate file: `./config/{development|production|anything}.toml`

### Config from the environment

You can provide environment variable to define log level and override configuration:

* Log level: `RUST_LOG={debug|info|warn|error}`. Also it's possible to define the level for a specific module: `RUST_LOG=writer=debug` turns on debug logging for `writer` module.
* Override config: define variable with a prefix of APP. Eg:

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

## License

Apache-2.0

[Rust]:https://www.rust-lang.org/en-US/index.html
[TOML]:https://github.com/toml-lang/toml
