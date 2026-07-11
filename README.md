# toonq — jq for TOON

CLI tool for querying, filtering, inspecting, and converting [TOON](https://toonformat.dev) (Token-Oriented Object Notation) files — the equivalent of `jq` for TOON data. Built on [jaq](https://github.com/01mf02/jaq), a Rust reimplementation of jq — see [Differences from jq](#differences-from-jq) for limitations.

```
toonq -f '.[] | select(.close > 10000) | {date, close}' data.toon
toonq --extract close data.toon
toonq --slurp --count session.jsonl
toonq -n -f '[range(5)]'                          # generate without input
toonq -f '.[] | select(.age > $min)' --argjson min 21 data.toon
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

# Query with jq syntax (via jaq engine)
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

`-f / --filter` accepts jq syntax via the [jaq](https://github.com/01mf02/jaq) engine (not 100% jq — see [differences](#differences-from-jq)):

```bash
toonq -f '.[] | select(.close > 100)' data.toon
toonq -f 'sort_by(-.date) | .[0:5] | {date, close}' data.toon
toonq -f 'group_by(.currency) | .[] | {currency: .[0].currency, count: length}' portfolio.toon
```

### Variables (`--arg`, `--argjson`)

Pass values into filters via named variables:

```bash
# --arg: string value
toonq -f '.[] | select(.name == $target)' --arg target alice data.toon

# --argjson: parsed JSON (numbers, arrays, objects)
toonq -f '.[] | select(.age > $min)' --argjson min 21 data.toon
toonq -f '. + $extra' --argjson extra '[4,5,6]' data.toon

# Multiple variables
toonq -f '.[] | select(.age > $min and .role == $role)' \
  --argjson min 21 --arg role admin data.toon
```

### Null input (`-n`)

Run the filter once with `null` as input — no file needed. Useful for generating data:

```bash
toonq -n -f '[range(5)]'                      # [0,1,2,3,4]
toonq -n -f '$greeting + ", world!"' --arg greeting Hello -r
```

### Raw output (`-r`)

Print string results without quotes, one per line. Non-string values stay compact JSON:

```bash
toonq -f '.[].name' -r data.toon              # alice
                                               # bob
toonq -f '.[].price' -r data.toon             # 100.5
```

### Multiple input files

With multiple files, the first is the filter input (`.`), the rest are available via `input`/`inputs`:

```bash
toonq -f '. + inputs' a.json b.json           # concatenate arrays
toonq -f '[inputs]' a.json b.json             # collect extras into array
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

## Differences from jq

toonq uses **jaq**, not the original `jq` (C). Modern jaq (3.x) implements nearly all of the jq language:

| Feature | jq | jaq / toonq |
|---------|:--:|:-----------:|
| Basic syntax (pipe, select, map, sort_by, group_by) | ✅ | ✅ |
| Object/array indexing and slicing | ✅ | ✅ |
| Variable binding (`as $x`) | ✅ | ✅ |
| External variables (`--arg`, `--argjson`) | ✅ | ✅ |
| Null input (`-n`) | ✅ | ✅ |
| Raw output (`-r`) | ✅ | ✅ |
| Multiple inputs (`input`, `inputs`) | ✅ | ✅ |
| `try` / `catch` | ✅ | ✅ |
| `foreach` | ✅ | ✅ |
| `walk` | ✅ | ✅ |
| `transpose` | ✅ | ✅ |
| `@csv`, `@tsv`, `@json` format strings | ✅ | ✅ |
| Format strings (`"\(.x)"`) | ❌ | ✅ (jaq extension) |
| Modules (`include`, `import` from file) | ✅ | ❌ |
| `input_filename` | ✅ | ❌ |

The only notable gaps are module loading from external files and `input_filename` (jaq implements the latter only in its CLI, not in the embedded library).

Full list: [jaq differences from jq](https://github.com/01mf02/jaq#differences-between-jaq-and-jq).

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
- [jaq differences from jq](https://github.com/01mf02/jaq#differences-between-jaq-and-jq) — upstream jaq limitations

## License

ISC
