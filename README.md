# toonq — jq for TOON

CLI tool for querying, filtering, inspecting, and converting [TOON](https://toonformat.dev) (Token-Oriented Object Notation) files — the equivalent of `jq` for TOON data.

```
toonq -f '.[] | select(.close > 10000) | {date, close}' data.toon
toonq --stats data.toon
toonq --to json data.toon | jq '. | length'
```

## Installation

```bash
# Prerequisite: jaq (jq engine in Rust)
cargo install jaq

# Install toonq
cargo install toonq
```

Or from source:

```bash
git clone https://github.com/.../toonq.git
cd toonq
cargo install --path .
```

## Quick start

```bash
# Inspect a TOON file
toonq --head 5 data.toon              # First 5 records
toonq --count data.toon               # How many records?
toonq --schema data.toon              # What fields and types?
toonq --stats data.toon               # TOON vs JSON size comparison

# Query with jq syntax
toonq -f '.[] | select(.close > 100)' data.toon
toonq -f 'sort_by(-.sharpe) | .[0:5]' metrics.toon

# Convert formats
toonq --to json data.toon             # TOON → JSON
toonq --from json data.json           # JSON → TOON

# Pipeline
cat data.toon | toonq -f '.[0:10]' | toonq --stats
toonq --to json data.toon | jq 'group_by(.currency)'
```

## Features

### Inspection

| Flag | Description |
|------|-------------|
| `--head N` | First N records (= jq `.[0:N]`) |
| `--tail N` | Last N records (= jq `.[-N:]`) |
| `--count` | Number of records |
| `--schema` | Field names and types |
| `--stats` | Token statistics (TOON vs JSON savings) |

### Queries

`-f / --filter` accepts full jq syntax via the [jaq](https://github.com/01mf02/jaq) engine:

```bash
toonq -f '.[] | select(.close > 100)' data.toon
toonq -f 'sort_by(-.date) | .[0:5] | {date, close}' data.toon
toonq -f 'group_by(.currency) | .[] | {currency: .[0].currency, count: length}' portfolio.toon
```

### Format conversion

| Flag | Description |
|------|-------------|
| `--to json` | Output as pretty-printed JSON |
| `--to toon` | Output as TOON (default) |
| `--to raw` | Raw compact JSON from jaq (for pipelines) |
| `--from json` | Read JSON input |
| `--from toon` | Read TOON input |
| `--from auto` | Auto-detect by file extension (default) |

### Pipelines

`toonq` reads from stdin and writes to stdout by default:

```bash
toonq -f 'filter' data.toon | toonq --head 3
toonq --to json data.toon | jq '. | length'
cat data.toon | toonq --count
```

## Requirements

- [jaq](https://github.com/01mf02/jaq) v3.0+ — `cargo install jaq`
- Rust toolchain for building from source

## How it works

```
TOON → serde_toon (Rust) → JSON → jaq -c subprocess → JSON → serde_toon → TOON
         native parser    ↑       jq-compatible engine  ↑      native output
                          └────── compact output ───────┘
```

`toonq` uses the `jaq` binary as a subprocess (not as a library). This gives full jq language compatibility without fighting the `jaq-core` API. TOON ↔ JSON conversion happens natively in Rust via `serde_toon_format`.

## License

ISC
