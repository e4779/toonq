# Why toonq converts Val manually (60 lines of Rust)

## The problem

We need to apply jq filters to TOON data and get TOON back:

```
TOON → jq filter → TOON
```

No JSON intermediary, no subprocess, no external dependencies.

## Three approaches, two dead ends, one solution

### Approach 1: jaq as subprocess

```
TOON → serde_toon → JSON string → jaq -c subprocess → JSON string → serde_toon → TOON
```

**Status:** Worked, but ugly. JSON serialization roundtrip, external dependency on `jaq` or `jq` binary, two `< / > pipe` boundaries.

### Approach 2: jaq-all as native library

`jaq-all` v0.1 is a high-level crate wrapping `jaq-core`. It provides:

```rust
let filter = jaq_all::data::compile(".[] | select(.close > 100)")?;
jaq_all::data::run(&runner, &filter, vars, inputs, |v| { ... });
```

The `jaq` binary itself uses this crate internally.

**Problem:** `jaq-all` operates on `jaq_json::Val`, not `serde_json::Value`. We need to convert between them. The obvious path — serde roundtrip — fails:

```rust
// DOES NOT COMPILE:
let jaq_val: jaq_json::Val = serde_json::from_value(json_val)?;
// Error: trait `serde_core::Deserialize` is not implemented for `Val`
```

But wait — `jaq-json` HAS a `serde` feature that enables `serde_core::Deserialize` for `Val`. Why doesn't it work?

### The serde / serde_core split

In 2025, `serde` was split into two crates:

| Crate | Contains |
|-------|----------|
| `serde_core` | `Serialize` and `Deserialize` traits only |
| `serde` | `serde_core` + `#[derive(Serialize, Deserialize)]` |

Both version 1.0.228. Same repository ([serde-rs/serde](https://github.com/serde-rs/serde)). `serde` re-exports `serde_core` via `pub use serde_core::*`.

**Key facts:**
- `serde_json` depends on `serde` (which re-exports `serde_core`)
- `jaq-json` depends on `serde_core` directly
- Both resolve to the same `serde_core` version (1.0.228) in the dependency graph
- `serde::Serialize` IS `serde_core::Serialize` — the compiler DOES unify re-exported traits

**Verified with experiment** ([test-cross2](https://gitverse.ru/e4779/toonq)):

```rust
// A local crate depending ONLY on `serde_core`:
struct MyType;
impl serde_core::Serialize for MyType { ... }

// In the binary depending on `serde_json`:
serde_json::to_value(MyType).unwrap(); // ✅ COMPILES
```

The Rust compiler correctly unifies `serde::Serialize` = `serde_core::Serialize`. The trait boundary is NOT the problem.

### The real problem: jaq_json::Val doesn't implement Serialize

Examination of `jaq-json` v2.0.0 source (`src/serde.rs`):

```rust
// jaq-json/src/serde.rs — the serde feature enables:
#[cfg(feature = "serde")]
mod serde;

// Inside serde.rs:
impl<'de> Deserialize<'de> for Val {  // ← ONLY Deserialize
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Val, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

// Serialize for Val — NOT IMPLEMENTED
```

Even with the `serde` feature enabled:
- ✅ `Val: serde_core::Deserialize` — can deserialize INTO Val
- ❌ `Val: serde_core::Serialize` — CANNOT serialize FROM Val

This is a deliberate limitation in `jaq-json`. The crate provides [`Val::to_string()`](https://docs.rs/jaq-json/2.0.0/jaq_json/struct.Val.html#impl-Display-for-Val) via the `Display` trait instead of `Serialize`.

### Why a forked serde_toon wouldn't help

Even if `serde_toon` used `serde_core` instead of `serde`:

```
TOON → serde_toon_core::Deserializer → jaq_json::Val (deserialize works ✅)
                                        ↓
                                   jaq-all filter
                                        ↓
                                   jaq_json::Val
                                        ↓
                                   ??? ← CANNOT serialize ❌
```

We can deserialize TOON directly into `Val`, but we cannot serialize `Val` back to TOON because `Serialize` is not implemented.

## Approach 3: Manual conversion (the solution)

Since `serde` traits are not the bottleneck, and the real issue is the missing `Serialize` impl, we bypass serde entirely:

```rust
// serde_json::Value → jaq_json::Val (deserialization)
fn json_to_jaq(v: &Value) -> JVal {
    match v {
        Value::Null => JVal::Null,
        Value::Bool(b) => JVal::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                JVal::Num(Num::Int(i as isize))
            } else if let Some(f) = n.as_f64() {
                JVal::Num(Num::Float(f))
            } else { JVal::Null }
        }
        Value::String(s) => JVal::utf8_str(s.clone()),
        Value::Array(arr) => arr.iter().map(json_to_jaq).collect(),
        Value::Object(obj) => {
            JVal::obj(obj.iter()
                .map(|(k, v)| (JVal::utf8_str(k.clone()), json_to_jaq(v)))
                .collect())
        }
    }
}

// jaq_json::Val → serde_json::Value (serialization)
fn jaq_to_json(v: &JVal) -> Value {
    match v {
        JVal::Null => Value::Null,
        JVal::Bool(b) => Value::Bool(*b),
        JVal::Num(Num::Int(i)) => Value::Number((*i).into()),
        JVal::Num(Num::Float(f)) => Value::Number(
            serde_json::Number::from_f64(*f).unwrap_or(0.into())
        ),
        JVal::Num(n) => { // BigInt or Dec
            let s = n.to_string();
            s.parse::<i64>().map(Value::Number).unwrap_or(Value::String(s))
        }
        JVal::TStr(data) | JVal::BStr(data) =>
            Value::String(String::from_utf8_lossy(data).into()),
        JVal::Arr(arr) => Value::Array(arr.iter().map(jaq_to_json).collect()),
        JVal::Obj(obj) => {
            Value::Object(obj.iter()
                .map(|(k, v)| (val_to_raw_string(k), jaq_to_json(v)))
                .collect())
        }
    }
}
```

**Nuances discovered during implementation:**

| Issue | Resolution |
|-------|-----------|
| `Val::to_string()` returns JSON-encoded string (with quotes) | Use `Val::as_bytes()` to extract raw bytes for TStr/BStr |
| `Num::as_f64()` and `Num::is_int()` are `pub(crate)` | Pattern-match `Num` variants directly: `Num::Int(i)`, `Num::Float(f)`, `Num::BigInt(bi)`, `Num::Dec(s)` |
| `Val::Arr(Rc<Vec<Val>>)` — not plain `Vec` | `FromIterator` works: `arr.iter().map(...).collect()` |
| Object keys are `JVal` (can be non-string) | Helper `val_to_raw_string()` extracts string keys; non-string keys fall back to `Display` |

## Final architecture

```
TOON → serde_toon → serde_json::Value → json_to_jaq() → jaq_json::Val
                                                            ↓
                                                       jaq-all filter
                                                            ↓
TOON ← serde_toon ← serde_json::Value ← jaq_to_json() ← jaq_json::Val
```

- Zero JSON serialization roundtrips
- Zero subprocesses
- Zero external runtime dependencies
- 60 lines of conversion code, well-tested
- `serde_json::Value` is used as intermediate because `serde_toon` produces it and we need it for `--json`, `--stats`, and inspection commands

## Key takeaways

1. **The `serde` / `serde_core` split is NOT the bottleneck.** The Rust compiler correctly unifies re-exported traits from the same crate version. [Verified experimentally](https://gitverse.ru/e4779/toonq).

2. **`jaq_json::Val` doesn't implement `Serialize`.** Only `Deserialize`. Even with the `serde` feature enabled. This is the real limitation.

3. **Manual conversion is the correct solution.** Not a workaround for a compiler limitation — a necessary bridge that provides missing `Serialize` functionality for `jaq_json::Val`.

4. **A forked `serde_toon` would not help.** It would fix deserialization (TOON → Val) but serialization (Val → TOON) would still be impossible.

## References

- [serde-rs/serde](https://github.com/serde-rs/serde) — serde split into `serde` + `serde_core`
- [serde#1465](https://github.com/serde-rs/serde/issues/1465) — "There is no supported way to re-export Serialize, Deserialize"
- [01mf02/jaq](https://github.com/01mf02/jaq) — jaq (jq in Rust)
- [jaq-json v2.0.0](https://crates.io/crates/jaq-json) — `Val` type; `serde` feature enables `serde_core` support (Deserialize only)
- [jaq-all v0.1.0](https://crates.io/crates/jaq-all) — high-level filter execution wrapper
- [serde_toon_format v0.1.2](https://crates.io/crates/serde_toon_format) — TOON serde implementation
- [bnomei/serde_toon](https://github.com/bnomei/serde_toon) — serde_toon source

## Experimental evidence

The `test-cross2` experiment (in toonq's git history, `native-filter` branch) demonstrates:

1. A type implementing `serde_core::Serialize` IS accepted by `serde_json::to_value()` — traits unify correctly.
2. `jaq_json::Val` does NOT implement `serde_core::Serialize` — the serde module only provides `Deserialize`.
3. `serde` re-exports `serde_core::*` — `serde::Serialize` and `serde_core::Serialize` are the same trait in Rust 1.95+.
