# PLAN.md — toonq

## Status: v0.2.0 ✅

495 lines of Rust, 1.8MB binary, 16 tests, 0 runtime dependencies.

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

## Files

- `src/main.rs` — 495 lines
- `test.sh` — 16 tests
- `docs/serde-research.md` — serde/serde_core deep dive
- `docs/recipes.md` — real-world workflows
