# TODO.md — toonq

## Phase 1-3: Core ✅
- [x] Cargo project, clap CLI, TOON ↔ Value
- [x] jaq filter (native jaq-all library)
- [x] TOON / JSON / raw output
- [x] stdin/stdout pipelines

## Phase 2: Inspection ✅
- [x] --head / --tail / --count / --schema / --stats

## Phase 3: Polish ✅
- [x] --from json + auto-detect (.toon, .json, .jsonl)
- [x] --to raw for non-filter operations
- [x] 11 tests

## Phase 4: JSONL + Truncate ✅
- [x] --slurp (JSONL → array)
- [x] --truncate N
- [x] Auto-detect JSONL on parse failure

## Phase 5: Extract ✅
- [x] --extract FIELD (field name)
- [x] --extract INDEX (array index)
- [x] --extract "0,2,8" (comma-separated)
- [x] 16 tests

## Backlog

- [ ] `cargo publish` to crates.io
- [ ] GitHub/GitVerse mirror
- [ ] CI (GitHub Actions)
- [ ] `--color` flag
