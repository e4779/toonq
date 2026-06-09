# toonq — jq for TOON

CLI tool for querying, filtering, inspecting, and converting [TOON](https://toonformat.dev) (Token-Oriented Object Notation) files — the equivalent of `jq` for TOON data.

```
toonq -f '.[] | select(.close > 10000) | {date, close}' data.toon
toonq --extract close data.toon
toonq --slurp --count session.jsonl
```

## Installation

```bash
cargo install toonq
```

No external dependencies. Everything (jaq engine, TOON parser, JSONL support) is compiled into the binary.

## Quick start

```bash
# Inspect
toonq --head 5 data.toon              # First 5 records
toonq --count data.toon               # How many records?
toonq --schema data.toon              # What fields and types?
toonq --stats data.toon               # TOON vs JSON size comparison

# Query with full jq syntax
toonq -f '.[] | select(.close > 100)' data.toon
toonq -f 'sort_by(-.sharpe) | .[0:5]' metrics.toon

# Extract fields (one call for all records)
toonq --extract text chat.json        # All messages
toonq --extract "0,2,8" chat.json     # Specific indices

# JSONL support
toonq --slurp --count data.jsonl      # Parse line-delimited JSON
toonq --slurp --truncate 80 --head 3 data.jsonl  # With truncation

# Convert
toonq --to json data.toon
toonq --from json data.json

# Pipeline
cat data.toon | toonq -f '.[0:10]' | toonq --stats
```

## Features

### Inspection

| Flag | Description |
|------|-------------|
| `--head N` | First N records |
| `--tail N` | Last N records |
| `--count` | Record count |
| `--schema` | Field names and types |
| `--stats` | Token statistics (TOON vs JSON savings) |
| `--truncate N` | Truncate string fields to N chars |

### Queries

`-f / --filter` accepts full jq syntax via the [jaq](https://github.com/01mf02/jaq) engine:

```bash
toonq -f '.[] | select(.close > 100)' data.toon
toonq -f 'sort_by(-.date) | .[0:5] | {date, close}' data.toon
toonq -f 'group_by(.currency) | .[] | {currency: .[0].currency, count: length}' portfolio.toon
```

### Field extraction

`--extract` pulls values by field name or array index:

```bash
toonq --extract close data.toon       # All close prices
toonq --extract role chat.json        # All roles
toonq --extract 0 chat.json           # First record
toonq --extract "0,2,8" chat.json     # Multiple records by index
```

### JSONL support

`--slurp` reads line-delimited JSON as an array. Auto-detected when JSON parsing fails:

```bash
toonq --slurp --count data.jsonl
toonq --from json --count data.jsonl  # Auto-detect
```

### Format conversion

| Flag | Description |
|------|-------------|
| `--to json` | Output as pretty-printed JSON |
| `--to toon` | Output as TOON (default) |
| `--to raw` | Compact JSON for pipelines |
| `--from json` | Read JSON input |
| `--from auto` | Auto-detect by file extension |

### Pipelines

```bash
toonq -f 'filter' data.toon | toonq --head 3
toonq --to json data.toon | jq '. | length'
cat data.toon | toonq --count
```

## How it works

```
TOON → serde_toon → jaq-all (native lib) → serde_toon → TOON
       TOON parser    jq-compatible engine    TOON encoder
```

`toonq` uses `jaq-all` — the same engine that powers the `jaq` binary — directly in-process. No subprocess, no JSON roundtrip, no runtime dependencies. See [docs/serde-research.md](docs/serde-research.md) for the architectural deep-dive.

## Documentation

- [docs/recipes.md](docs/recipes.md) — real-world workflows (chat logs, JSONL, financial data)
- [docs/serde-research.md](docs/serde-research.md) — why manual Val conversion, serde/serde_core investigation
- `toonq --help` — all flags and basic examples

## License

ISC
