# rust_parser

A toy Rust project (crate `rust_parser`) that implements parsing of the HDFS dataset into a CSV format with template annotation.
This is a performance improvement exercise over the original Python implementation, which can be found in this [repository](https://github.com/ait-aecid/anomaly-detection-log-datasets/blob/main/hdfs_parse.py).

The binary expects an input log file "sorted.log" and a file containing templates "templates.csv" in the current working directory when executed. It produces an output file "parsed_rust.csv".

## Build

This project uses Cargo. To build in debug or release:

```bash
# build debug
cargo build

# build release
cargo build --release
```

## Run

Run the binary produced in `target/debug` or `target/release`:

```bash
# run with cargo (debug)
cargo run

# run with cargo (release)
cargo run --release
```
