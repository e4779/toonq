//! jq for TOON — query, filter, inspect, and convert Token-Oriented Object Notation files.
//!
//! Reads TOON (or JSON) data, applies jq filters via the `jaq` engine,
//! and outputs results in TOON or JSON format.
//!
//! Pipe-friendly: reads from stdin by default, writes to stdout.

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::Parser;
use serde_json::Value;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

/// jq for TOON — query, filter, inspect, and convert Token-Oriented Object Notation files.
///
/// Reads TOON (or JSON) data, applies jq filters via the `jaq` engine,
/// and outputs results in TOON or JSON format.
///
/// Pipe-friendly: reads from stdin by default, writes to stdout.
#[derive(Parser)]
#[command(
    name = "toonq",
    version = VERSION,
    about = "jq for TOON — query, filter, inspect, and convert TOON files",
    long_about = "jq for TOON — query, filter, inspect, and convert Token-Oriented Object Notation files.\n\nReads TOON (or JSON) data, applies jq filters via the `jaq` engine, and outputs results in TOON or JSON format. Pipe-friendly: reads from stdin by default, writes to stdout.",
    after_help = "EXAMPLES:\n  toonq --head 5 data.toon\n  toonq --count data.toon\n  toonq --schema data.toon\n  toonq --stats data.toon\n  toonq -f '.[] | select(.close > 100)' data.toon\n  toonq --extract close data.toon\n  toonq --slurp --count data.jsonl\n  toonq --to json data.toon\n  toonq --from json data.json\n  toonq -n -f '[range(5)]'\n  toonq -f '.[].name' -r data.toon\n  toonq -f '.[] | select(.age > $min)' --argjson min 21 data.toon\n\nSee docs/recipes.md for real-world workflows.",
)]
struct Cli {
    /// jq filter expression to apply to input data.
    /// Uses the full jq language via the `jaq` engine.
    #[arg(short = 'f', long = "filter", verbatim_doc_comment)]
    filter: Option<String>,

    /// Input file(s). Omit or use "-" for stdin.
    /// With -f: first file is the filter input (.), rest available via `inputs`.
    #[arg(value_name = "FILE")]
    inputs: Vec<PathBuf>,

    /// Input format: toon, json, or auto (detect by file extension).
    /// Default: auto for files, toon for stdin.
    #[arg(long = "from", default_value = "auto", verbatim_doc_comment)]
    input_format: String,

    /// Output format: toon (pretty-printed), json (pretty-printed), or raw (compact JSON).
    /// Use `raw` for pipelines where you need compact machine-readable output.
    /// For unquoted string output (like jq -r), use -r/--raw-output instead.
    #[arg(long = "to", default_value = "toon", verbatim_doc_comment)]
    output_format: String,

    /// Show only the first N records (equivalent to jq `.[0:N]`).
    #[arg(long, verbatim_doc_comment)]
    head: Option<usize>,

    /// Show only the last N records (equivalent to jq `.[-N:]`).
    #[arg(long, verbatim_doc_comment)]
    tail: Option<usize>,

    /// Show the number of records in the top-level array.
    #[arg(long, verbatim_doc_comment)]
    count: bool,

    /// Show the schema: field names and their types.
    /// Inspects the first record to determine types.
    #[arg(long, verbatim_doc_comment)]
    schema: bool,

    /// Show token statistics comparing TOON and JSON sizes.
    /// Reports byte counts, estimated tokens, and savings percentage.
    #[arg(long, verbatim_doc_comment)]
    stats: bool,

    /// Slurp JSONL input into an array (like `jq -s '.'`).
    /// Auto-detected when JSON parsing fails.
    #[arg(long = "slurp", verbatim_doc_comment)]
    slurp: bool,

    /// Extract values by field name or array index.
    /// Comma-separated for multiple: --extract "0,2,5" or --extract "name,age".
    /// Shortcut for `-f 'map(.FIELD)'` or `-f '.[INDEX]'`.
    #[arg(long = "extract", verbatim_doc_comment)]
    extract: Option<String>,

    /// Truncate string fields to N characters, appending "…" if truncated.
    /// Useful for inspecting data with long text fields.
    #[arg(long = "truncate", verbatim_doc_comment)]
    truncate: Option<usize>,

    /// Pass a string variable: --arg name value (becomes $name == "value").
    /// Multiple: --arg a 1 --arg b 2
    #[arg(long = "arg", value_names = &["NAME", "VALUE"], num_args = 2, verbatim_doc_comment)]
    args: Vec<String>,

    /// Pass a JSON variable: --argjson name '{"k": 1}' (becomes $name == parsed JSON).
    /// Multiple: --argjson a 1 --argjson b '[1,2]'
    #[arg(long = "argjson", value_names = &["NAME", "JSON"], num_args = 2, verbatim_doc_comment)]
    argjson: Vec<String>,

    /// Don't read input: filter runs once with `null` as input.
    /// Useful for generating data: toonq -n -f '[range(5)]'.
    #[arg(short = 'n', long = "null-input", verbatim_doc_comment)]
    null_input: bool,

    /// Raw output: strings printed without quotes, each on its own line.
    /// Non-string values print as compact JSON.
    #[arg(short = 'r', long = "raw-output", verbatim_doc_comment)]
    raw_output: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let has_operation = cli.filter.is_some()
        || cli.schema || cli.count
        || cli.head.is_some() || cli.tail.is_some()
        || cli.stats || cli.slurp || cli.truncate.is_some() || cli.extract.is_some()
        || cli.null_input
        || cli.output_format != "toon"
        || cli.input_format != "auto";

    if !has_operation {
        anyhow::bail!(
            "No operation specified. Use -f/--filter for jq queries, \
             or --head/--tail/--count/--schema/--stats for inspection.\n\
             Example: toonq --head 5 data.toon\n\
             Example: toonq -f '.[] | select(.close > 100)' data.toon"
        );
    }

    // Collect variables from --arg (string) and --argjson (parsed JSON).
    let (var_names, var_values) = collect_vars(&cli)?;

    // With -n, we don't read any input file.
    if cli.null_input {
        if let Some(filter) = &cli.filter {
            let result = run_jaq_native(None, filter, &cli, &var_names, &var_values)?;
            if !cli.raw_output {
                output_value(&result, &cli.output_format)?;
            }
            return Ok(());
        }
        anyhow::bail!("--null-input requires -f/--filter");
    }

    // Read and parse the primary input (first file or stdin).
    let input_path = cli.inputs.first().map(|p| p.as_path());
    let input_text = read_input(input_path)?;
    if input_text.trim().is_empty() {
        return Ok(());
    }

    let format = if cli.slurp {
        "json".to_string()
    } else {
        detect_format(&cli.input_format, input_path)
    };

    // Parse input — try JSON/TOON first, fall back to JSONL if slurp or parse fails.
    let mut value: Value = parse_input(&input_text, &format, cli.slurp)?;

    // Read additional input files (for `input`/`inputs` inside filters).
    let extra_inputs: Vec<Value> = cli.inputs.iter().skip(1)
        .map(|p| {
            let text = read_input(Some(p))?;
            let fmt = detect_format(&cli.input_format, Some(p));
            parse_input(&text, &fmt, cli.slurp)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    // Apply truncation early (affects --head, --tail, --schema output too).
    if let Some(max_len) = cli.truncate {
        truncate_strings(&mut value, max_len);
    }

    if cli.stats {
        print_stats(&input_text, &value);
        return Ok(());
    }

    let result = if cli.schema {
        schema(&value)
    } else if cli.count {
        count(&value)
    } else if let Some(n) = cli.head {
        head(&value, n)
    } else if let Some(n) = cli.tail {
        tail(&value, n)
    } else if let Some(field) = &cli.extract {
        extract_field(&value, field)
    } else if let Some(filter) = &cli.filter {
        run_jaq_native(
            Some((&value, &extra_inputs)), filter, &cli,
            &var_names, &var_values,
        )?
    } else {
        value.clone()
    };

    // For filter operations with raw output, run_jaq_native already printed directly.
    if cli.filter.is_some() && cli.raw_output {
        return Ok(());
    }

    if cli.output_format != "raw" {
        output_value(&result, &cli.output_format)?;
    } else {
        // Raw mode for non-filter operations: print compact JSON.
        println!("{}", serde_json::to_string(&result)?);
    }

    Ok(())
}

// ── Input ──────────────────────────────────────────────────────────────────

fn read_input(path: Option<&std::path::Path>) -> anyhow::Result<String> {
    let is_stdin = path.is_none_or(|p| p.to_str() == Some("-"));
    if is_stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)
            .context("Failed to read stdin")?;
        return Ok(buf);
    }
    let p = path.unwrap();
    fs::read_to_string(p)
        .with_context(|| format!("Failed to read {}", p.display()))
}

fn detect_format(explicit: &str, path: Option<&std::path::Path>) -> String {
    if explicit != "auto" {
        return explicit.to_string();
    }
    if let Some(p) = path {
        match p.extension().and_then(|e| e.to_str()) {
            Some("toon") => return "toon".into(),
            Some("json") | Some("jsonl") => return "json".into(),
            _ => {}
        }
    }
    "toon".into()
}

/// Parse input text into a Value using the detected format, with JSONL fallback.
fn parse_input(text: &str, format: &str, slurp: bool) -> anyhow::Result<Value> {
    match format {
        "toon" => serde_toon::from_str(text)
            .context("Failed to parse TOON input"),
        "json" => match serde_json::from_str(text) {
            Ok(v) => Ok(v),
            Err(_) if slurp => slurp_jsonl(text),
            Err(e) => {
                // Auto-detect: try JSONL as fallback.
                match slurp_jsonl(text) {
                    Ok(v) => {
                        eprintln!("Note: detected JSONL, parsing as array (use --slurp to suppress)");
                        Ok(v)
                    }
                    Err(_) => Err(e).context("Failed to parse JSON input. Not valid JSON or JSONL."),
                }
            }
        },
        other => bail!("Unknown input format: {other}"),
    }
}

/// Collect variables from --arg (strings) and --argjson (parsed JSON values).
/// Returns (names, values) in matching order for compile + runtime injection.
fn collect_vars(cli: &Cli) -> anyhow::Result<(Vec<String>, Vec<Value>)> {
    let mut names = Vec::new();
    let mut values = Vec::new();

    for chunk in cli.args.chunks_exact(2) {
        names.push(chunk[0].clone());
        values.push(Value::String(chunk[1].clone()));
    }

    for chunk in cli.argjson.chunks_exact(2) {
        let val: Value = serde_json::from_str(&chunk[1])
            .with_context(|| format!("--argjson {} invalid JSON: {}", chunk[0], chunk[1]))?;
        names.push(chunk[0].clone());
        values.push(val);
    }

    Ok((names, values))
}

// ── Native jaq filter (no subprocess, no JSON roundtrip) ──────────────────

use jaq_all::json::Val as JVal;

fn json_to_jaq(v: &Value) -> JVal {
    match v {
        Value::Null => JVal::Null,
        Value::Bool(b) => JVal::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= isize::MIN as i64 && i <= isize::MAX as i64 {
                    // machine integer range
                    JVal::Num(jaq_all::json::Num::Int(i as isize))
                } else {
                    JVal::Num(jaq_all::json::Num::from_integral(i))
                }
            } else if let Some(f) = n.as_f64() {
                JVal::Num(jaq_all::json::Num::Float(f))
            } else {
                JVal::Null
            }
        }
        Value::String(s) => JVal::utf8_str(s.clone()),
        Value::Array(arr) => arr.iter().map(json_to_jaq).collect(),
        Value::Object(obj) => {
            let pairs: Vec<(JVal, JVal)> = obj.iter()
                .map(|(k, v)| (JVal::utf8_str(k.clone()), json_to_jaq(v)))
                .collect();
            JVal::obj(pairs.into_iter().collect())
        }
    }
}

/// Extract a raw Rust string from a JVal (for object keys).
fn val_to_raw_string(v: &JVal) -> String {
    match v {
        JVal::BStr(data) | JVal::TStr(data) => String::from_utf8_lossy(data).into_owned(),
        _ => v.to_string(),
    }
}

fn jaq_to_json(v: &JVal) -> Value {
    use jaq_all::json::Num;
    match v {
        JVal::Null => Value::Null,
        JVal::Bool(b) => Value::Bool(*b),
        JVal::Num(n) => match n {
            Num::Int(i) => Value::Number((*i).into()),
            Num::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            // BigInt and Dec: format as string, parse back
            _ => {
                let s = n.to_string();
                if let Ok(i) = s.parse::<i64>() {
                    Value::Number(i.into())
                } else if let Ok(f) = s.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                } else {
                    Value::String(s)
                }
            }
        },
        JVal::BStr(data) | JVal::TStr(data) => {
            // Extract raw bytes, convert to UTF-8 string
            Value::String(String::from_utf8_lossy(data).into_owned())
        }
        JVal::Arr(arr) => Value::Array(arr.iter().map(jaq_to_json).collect()),
        JVal::Obj(obj) => {
            let map: serde_json::Map<String, Value> = obj.iter()
                .map(|(k, v)| (val_to_raw_string(k), jaq_to_json(v)))
                .collect();
            Value::Object(map)
        }
    }
}

/// Run a jaq filter against the input.
///
/// `input` is `(primary, extras)` or `None` for `--null-input`.
/// Extras are fed after the primary so `input`/`inputs` inside the filter
/// can pull them from the stream.
fn run_jaq_native(
    input: Option<(&Value, &[Value])>,
    filter_str: &str,
    cli: &Cli,
    var_names: &[String],
    var_values: &[Value],
) -> anyhow::Result<Value> {
    use jaq_all::data;
    use jaq_all::jaq_core::Vars;

    // Compile: use compile_with when we have variables so the compiler
    // registers $name references. Without variables, data::compile is fine.
    let filter = if var_names.is_empty() {
        data::compile(filter_str)
    } else {
        jaq_all::compile_with(filter_str, jaq_all::defs(), data::funs(), var_names)
    }
    .map_err(|reports| {
        let msgs: Vec<String> = reports.iter()
            .flat_map(|r| r.1.iter().map(|m| format!("{m:?}")))
            .collect();
        anyhow::anyhow!("jaq compile error: {}", msgs.join("\n"))
    })?;

    // Build the variable values vector (jaq Val type).
    let jaq_vars: Vec<JVal> = var_values.iter().map(json_to_jaq).collect();

    // Build the input iterator: primary value then extras.
    // For --null-input, we pass a dummy null; Runner.null_input replaces it.
    let jaq_inputs: Vec<JVal> = match input {
        None => vec![],
        Some((primary, extras)) => {
            let mut v = vec![json_to_jaq(primary)];
            v.extend(extras.iter().map(json_to_jaq));
            v
        }
    };

    let runner = data::Runner {
        null_input: cli.null_input,
        ..data::Runner::default()
    };

    let mut results: Vec<JVal> = Vec::new();

    data::run(
        &runner,
        &filter,
        Vars::new(jaq_vars),
        jaq_inputs.into_iter().map(Ok::<_, String>),
        |e| anyhow::anyhow!("jaq input error: {e}"),
        |v| {
            results.push(v.map_err(|e| anyhow::anyhow!("jaq error: {e}"))?);
            Ok(())
        },
    ).map_err(|e| anyhow::anyhow!("jaq execution error: {e}"))?;

    // Raw output: strings without quotes, non-strings as compact JSON, one per line.
    if cli.raw_output {
        for v in &results {
            match v {
                JVal::BStr(_) | JVal::TStr(_) => {
                    println!("{}", val_to_raw_string(v));
                }
                _ => {
                    let jv = jaq_to_json(v);
                    println!("{}", serde_json::to_string(&jv)?);
                }
            }
        }
        return Ok(Value::Null);
    }

    Ok(match results.len() {
        0 => Value::Null,
        1 => jaq_to_json(&results[0]),
        _ => Value::Array(results.iter().map(jaq_to_json).collect()),
    })
}

// ── Output ─────────────────────────────────────────────────────────────────

fn output_value(value: &Value, format: &str) -> anyhow::Result<()> {
    match format {
        "toon" => {
            let out = serde_toon::to_string(value)
                .context("Failed to encode TOON output")?;
            println!("{out}");
        }
        "json" => {
            let out = serde_json::to_string_pretty(value)?;
            println!("{out}");
        }
        "raw" => {
            let out = serde_json::to_string(value)?;
            println!("{out}");
        }
        other => bail!("Unknown output format: {other} (use 'toon', 'json', or 'raw')"),
    }
    Ok(())
}

// ── Inspection commands ────────────────────────────────────────────────────

fn schema(value: &Value) -> Value {
    match value {
        Value::Array(arr) if !arr.is_empty() => {
            if let Value::Object(obj) = &arr[0] {
                let mut fields = serde_json::Map::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), type_name(v));
                }
                return Value::Object(fields);
            }
        }
        _ => {}
    }
    Value::String("non-tabular data".into())
}

fn count(value: &Value) -> Value {
    let n = match value {
        Value::Array(arr) => arr.len(),
        _ => 1,
    };
    Value::Number(n.into())
}

fn extract_field(value: &Value, field: &str) -> Value {
    // Comma-separated: extract multiple indices/fields
    if field.contains(',') {
        let keys: Vec<&str> = field.split(',').map(|s| s.trim()).collect();
        let results: Vec<Value> = keys.iter()
            .map(|k| extract_single(value, k))
            .collect();
        return Value::Array(results);
    }
    extract_single(value, field)
}

fn extract_single(value: &Value, field: &str) -> Value {
    match value {
        Value::Array(arr) => {
            // Numeric index
            if let Ok(idx) = field.parse::<usize>() {
                return arr.get(idx).cloned().unwrap_or(Value::Null);
            }
            // Field name from each object
            Value::Array(arr.iter()
                .filter_map(|v| v.get(field).cloned())
                .collect())
        }
        Value::Object(obj) => obj.get(field).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn head(value: &Value, n: usize) -> Value {
    match value {
        Value::Array(arr) => Value::Array(arr.iter().take(n).cloned().collect()),
        other => other.clone(),
    }
}

fn tail(value: &Value, n: usize) -> Value {
    match value {
        Value::Array(arr) => {
            let len = arr.len();
            let start = len.saturating_sub(n);
            Value::Array(arr[start..].to_vec())
        }
        other => other.clone(),
    }
}

fn type_name(v: &Value) -> Value {
    Value::String(match v {
        Value::Null => "null".into(),
        Value::Bool(_) => "bool".into(),
        Value::Number(n) => {
            if n.is_f64() { "float".into() } else { "integer".into() }
        }
        Value::String(_) => "string".into(),
        Value::Array(_) => "array".into(),
        Value::Object(_) => "object".into(),
    })
}

// ── Stats ──────────────────────────────────────────────────────────────────

fn print_stats(input_text: &str, value: &Value) {
    let toon_bytes = input_text.len();
    let json_text = serde_json::to_string(value).unwrap_or_default();
    let json_bytes = json_text.len();

    let record_count = match value {
        Value::Array(arr) => arr.len(),
        _ => 1,
    };

    let toon_tokens = estimate_tokens(input_text);
    let json_tokens = estimate_tokens(&json_text);

    println!("Records:       {record_count}");
    println!("TOON bytes:    {toon_bytes}");
    println!("JSON bytes:    {json_bytes}");
    println!("TOON tokens:   ~{toon_tokens}");
    println!("JSON tokens:   ~{json_tokens}");
    if json_bytes > 0 {
        let byte_savings = 100.0 * (1.0 - toon_bytes as f64 / json_bytes as f64);
        println!("Byte savings:  {byte_savings:.1}%");
    }
    if json_tokens > 0 {
        let token_savings = 100.0 * (1.0 - toon_tokens as f64 / json_tokens as f64);
        println!("Token savings: {token_savings:.1}%");
    }
}

/// Estimate token count using GPT-style heuristics: words + punctuation.
/// This approximates BPE tokenization by counting whitespace-delimited words
/// as one token each, and punctuation/special characters as separate tokens.
fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0;
    let mut in_word = false;
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            if !in_word {
                tokens += 1;
                in_word = true;
            }
        } else {
            in_word = false;
            if !c.is_whitespace() {
                tokens += 1;
            }
        }
    }
    tokens
}

// ── JSONL support ─────────────────────────────────────────────────────────

/// Parse JSONL (one JSON value per line) into a JSON array.
fn slurp_jsonl(input: &str) -> anyhow::Result<Value> {
    let items: Result<Vec<Value>, _> = input
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect();
    let items = items.context("Failed to parse JSONL: each line must be valid JSON")?;
    if items.is_empty() {
        anyhow::bail!("JSONL input is empty");
    }
    Ok(Value::Array(items))
}

// ── String truncation ─────────────────────────────────────────────────────

/// Recursively truncate all string values to `max_len` characters.
/// Appends "…" if truncated.
fn truncate_strings(value: &mut Value, max_len: usize) {
    if max_len == 0 {
        return;
    }
    match value {
        Value::String(s) if s.len() > max_len => {
            let trunc = s.chars().take(max_len).collect::<String>();
            *s = format!("{trunc}…");
        }
        Value::Array(arr) => {
            for v in arr {
                truncate_strings(v, max_len);
            }
        }
        Value::Object(obj) => {
            for (_, v) in obj.iter_mut() {
                truncate_strings(v, max_len);
            }
        }
        _ => {}
    }
}
