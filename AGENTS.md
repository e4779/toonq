# AGENTS.md — toonq

Read README.md for features, API, and project structure.

## Architecture decisions (rationale)

1. **jaq-all as native library, not subprocess.** We embed `jaq-all` (the same engine as the `jaq` binary) directly. No external dependencies at runtime. Details: `docs/serde-research.md`.

2. **Manual Val conversion (60 lines).** `serde_json::Value` ↔ `jaq_json::Val` is done by hand because `jaq_json::Val` doesn't implement `Serialize`. The `serde`/`serde_core` split is a red herring — traits DO unify, `Serialize` simply doesn't exist. Details: `docs/serde-research.md`.

3. **No identity mode.** Empty `toonq file.toon` is an error. Format conversion (`--to json`) IS an operation.

## Gotchas

- `Val::to_string()` returns JSON representation (with quotes). Use `Val::as_bytes()` for raw string data.
- `Num::as_f64()` and `Num::is_int()` are `pub(crate)` — pattern-match variants directly.
- `Val::Arr` wraps `Rc<Vec<Val>>`, but `FromIterator` works for construction.
