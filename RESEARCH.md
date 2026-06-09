# RESEARCH.md — toonq

## The problem

Build a `jq`-equivalent for TOON — query, filter, inspect TOON files without JSON intermediaries.

## Ecosystem scan (June 2026)

### Existing candidates (non-working)

| Crate | Version | Description | Issue |
|-------|---------|-------------|-------|
| [`oq`](https://crates.io/crates/oq) | 0.1.0 | "Object Query — jq for JSON, YAML, TOML, TOON" | Doesn't compile (10 lifetime errors in jaq-json) |
| [`toon_ql`](https://crates.io/crates/toon_ql) | 0.0.2 | "A query language for Toon data" | Library-only, 0% docs, no CLI |

### TOON ecosystem

~180 crates on crates.io — all parsers/encoders. Zero query tools.

Official site (toonformat.dev) — CLI converter, playground, syntax highlighting only.

## Architecture evolution

### Attempt 1: jaq subprocess

```
TOON → JSON string → jaq -c subprocess → JSON string → TOON
```

**Verdict:** Worked, but JSON roundtrip + external dependency. Rejected.

### Attempt 2: jaq-all native library

`jaq-all` v0.1 — the same engine as the `jaq` binary, available as a Rust library.

**Blocked by:** `jaq_json::Val` uses `serde_core`, `serde_json::Value` uses `serde`. Same traits, different crate names — Rust compiler doesn't unify them.

### Attempt 3: Fork serde_toon on serde_core

Replace `serde` with `serde_core` in `serde_toon` → direct TOON → `jaq_json::Val`.

**Blocked by:** `jaq_json::Val` implements `Deserialize` but NOT `Serialize`. Even with the fork, we couldn't serialize Val back to TOON.

### Solution: Manual conversion (60 lines)

Bypass serde entirely. Convert between `serde_json::Value` and `jaq_json::Val` directly.

Full details: `docs/serde-research.md`

## Final architecture

```
TOON → serde_toon → serde_json::Value → json_to_jaq() → jaq_json::Val
                                                              ↓
                                                         jaq-all filter
                                                              ↓
TOON ← serde_toon ← serde_json::Value ← jaq_to_json() ← jaq_json::Val
```

- Zero JSON intermediaries
- Zero subprocesses
- Zero runtime dependencies
- 60 lines of conversion code, battle-tested

## Key finding

The `serde`/`serde_core` split is NOT the bottleneck — Rust correctly unifies re-exported traits. The real issue: `jaq_json::Val` only implements `Deserialize`, not `Serialize`. Manual conversion fills this gap.
