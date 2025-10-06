# rust_parser

A toy Rust project (crate `rust_parser`) that implements parsing of the HDFS dataset into a CSV format with template annotation.
This is a performance improvement exercise over the original Python implementation, which can be found in this [repository](https://github.com/ait-aecid/anomaly-detection-log-datasets/blob/main/hdfs_parse.py).

The binary expects several files present in the current working directory, see original repository for details on how to obtain them:

- `sorted.log` — input log file where each line is a log event (timestamps expected in the log format used by the HDFS dataset).
- `templates.csv` — templates, where each template is split into parts and stored using the repository's template encoding (the program parses each line, splitting on the template separators).
- `labels.csv` — mapping from block IDs to labels (e.g., Normal / Anomaly).

The program writes `parsed_rust.csv` containing parsed and annotated events.

## What it does (summary)

- Builds an [Aho-Corasick automaton](https://docs.rs/aho-corasick/) from the first part of every template to quickly find candidate templates in a log line.
- For each candidate, it searches the remaining template parts sequentially (using a subarray search) to ensure the entire template matches the line.
- Extracts a block identifier (`blk_id`) from matched lines (special-cased for template id 30 which may contain multiple block IDs).
- Parses timestamps from the log line (format: two-digit year + numeric month/day + time fields) into Unix timestamps.
- Looks up the block's label from `labels.csv` (labels use `N` or `A` in the CSV and are mapped to Normal/Anomaly for output).
- Writes one or more output rows per matched line to `parsed_rust.csv` with the resolved template id, block id, timestamp and label.

The implementation also collects template frequencies and periodically reorders candidate template checks to prefer more frequent templates (a heuristic to improve average matching time). A sort threshold increases multiplicatively during processing to reduce reorder frequency.

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
