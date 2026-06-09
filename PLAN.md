# PLAN.md — toonq

## Status: v0.2.4 ✅

534 lines of Rust (main.rs 504 + build.rs 34), 16 tests, 0 runtime dependencies.

Published on [crates.io](https://crates.io/crates/toonq). CI on every push to main and tag.

## Architecture

```
TOON → serde_toon → serde_json::Value → json_to_jaq() → jaq_json::Val
                                                              ↓
                                                         jaq-all filter
                                                              ↓
TOON ← serde_toon ← serde_json::Value ← jaq_to_json() ← jaq_json::Val
```

- `serde_toon` — native TOON parser/encoder
- `jaq-all` — jq engine, compiled into binary
- `json_to_jaq` / `jaq_to_json` — manual conversion (60 lines), bypasses serde/serde_core incompatibility
- Details: `docs/serde-research.md`

## Stack

| Crate | Purpose |
|-------|---------|
| `serde_toon_format` 0.1.2 | TOON ↔ serde_json::Value |
| `serde_json` 1 | Intermediate Value type |
| `jaq-all` 0.1 (+ formats) | jq engine (native library) |
| `jaq-json` 2 (+ serde) | Val type |
| `clap` 4 | CLI |
| `anyhow` 1 | Errors |

## Features

- Inspection: `--head`, `--tail`, `--count`, `--schema`, `--stats`
- Queries: `-f` (full jq syntax via jaq-all)
- Extract: `--extract FIELD`, `--extract INDEX`, `--extract "0,2,8"`
- JSONL: `--slurp`, auto-detect, `.jsonl` extension
- Truncation: `--truncate N`
- Format: `--to json`/`toon`/`raw`, `--from json`/`auto`
- Pipelines: stdin → stdout, chaining
- `--version` shows git hash (via build.rs + GIT_HASH file in CI)

## Files

- `src/main.rs` — 504 lines
- `build.rs` — 34 lines (git hash embedding)
- `test.sh` — 16 tests
- `docs/serde-research.md` — serde/serde_core deep dive
- `docs/recipes.md` — real-world workflows

## CI

Single workflow in `.gitverse/workflows/ci.yml`:
- **test**: git clone, cargo build, cargo test (on every push/PR to main)
- **publish**: version verification, dry-run, publish to crates.io (on tags `v*`)
