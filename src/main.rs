use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, bail};
use clap::Parser;
use serde_json::Value;
use serde_toon;

/// jq for TOON — query, filter, inspect, and convert Token-Oriented Object Notation files.
///
/// Reads TOON (or JSON) data, applies jq filters via the `jaq` engine,
/// and outputs results in TOON or JSON format.
///
/// Pipe-friendly: reads from stdin by default, writes to stdout.
#[derive(Parser)]
#[command(
    name = "toonq",
    version,
    about = "jq for TOON — query, filter, inspect, and convert TOON files",
    long_about = "jq for TOON — query, filter, inspect, and convert Token-Oriented Object Notation files.\n\nReads TOON (or JSON) data, applies jq filters via the `jaq` engine, and outputs results in TOON or JSON format. Pipe-friendly: reads from stdin by default, writes to stdout.",
    after_help = "EXAMPLES:\n  # Inspection\n  toonq --head 5 data.toon              # First 5 records\n  toonq --tail 3 data.toon              # Last 3 records\n  toonq --count data.toon               # Record count\n  toonq --schema data.toon              # Fields and types\n  toonq --stats data.toon               # Token statistics (TOON vs JSON)\n\n  # Queries (full jq syntax)\n  toonq -f '.[] | select(.close > 100)' data.toon\n  toonq -f 'sort_by(-.sharpe) | .[0:5]' metrics.toon\n  toonq -f 'group_by(.currency) | .[] | {key, count: length}' portfolio.toon\n\n  # Format conversion\n  toonq --to json data.toon             # TOON → JSON\n  toonq --from json data.json           # JSON → TOON\n  toonq -f '.[0:3]' --to raw data.toon  # Raw jaq output (compact JSON)\n\n  # Pipelines\n  toonq -f 'filter' data.toon | toonq --head 3\n  toonq --to json data.toon | jq '. | length'\n  cat data.toon | toonq --count\n\nRequires `jaq` or `jq` for filter execution (falls back to `jq` if `jaq` is not installed).",
)]
struct Cli {
    /// jq filter expression to apply to input data.
    /// Uses the full jq language via the `jaq` engine.
    #[arg(short = 'f', long = "filter", verbatim_doc_comment)]
    filter: Option<String>,

    /// Input file path.
    /// Omit or use "-" to read from stdin.
    /// Format is auto-detected by extension (.toon / .json).
    input: Option<PathBuf>,

    /// Input format: toon, json, or auto (detect by file extension).
    /// Default: auto for files, toon for stdin.
    #[arg(long = "from", default_value = "auto", verbatim_doc_comment)]
    input_format: String,

    /// Output format: toon (pretty-printed), json (pretty-printed), or raw (compact jaq output).
    /// Use `raw` for pipelines where you need compact machine-readable output.
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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Determine if we have any operation to perform
    let has_operation = cli.filter.is_some()
        || cli.schema || cli.count
        || cli.head.is_some() || cli.tail.is_some()
        || cli.stats
        || cli.output_format != "toon"  // format conversion is an operation
        || cli.input_format != "auto";   // explicit input format is an operation

    if !has_operation {
        anyhow::bail!(
            "No operation specified. Use -f/--filter for jq queries, \
             or --head/--tail/--count/--schema/--stats for inspection.\n\
             Example: toonq --head 5 data.toon\n\
             Example: toonq -f '.[] | select(.close > 100)' data.toon"
        );
    }

    // 1. Read input
    let input_text = read_input(cli.input.as_deref())?;
    if input_text.trim().is_empty() {
        return Ok(());
    }

    // 2. Detect format and parse → serde_json::Value
    let format = detect_format(&cli.input_format, cli.input.as_deref());
    let value: Value = match format.as_str() {
        "toon" => serde_toon::from_str(&input_text)
            .context("Failed to parse TOON input")?,
        "json" => serde_json::from_str(&input_text)
            .context("Failed to parse JSON input")?,
        other => bail!("Unknown input format: {other}"),
    };

    // 3. Stats mode — short-circuit (stats prints its own output)
    if cli.stats {
        print_stats(&input_text, &value);
        return Ok(());
    }

    // 4. Apply filter or inspection command
    let result = if cli.schema {
        schema(&value)
    } else if cli.count {
        count(&value)
    } else if let Some(n) = cli.head {
        head(&value, n)
    } else if let Some(n) = cli.tail {
        tail(&value, n)
    } else if let Some(filter) = &cli.filter {
        run_jaq_filter(&value, filter, cli.output_format.as_str())?
    } else {
        // Identity — format conversion only
        value.clone()
    };

    // 5. Output (only for non-raw mode; raw is handled inside run_jaq_filter)
    if cli.output_format != "raw" {
        output_value(&result, &cli.output_format)?;
    }

    Ok(())
}

// ── Input ──────────────────────────────────────────────────────────────────

fn read_input(path: Option<&std::path::Path>) -> anyhow::Result<String> {
    let is_stdin = path.map_or(true, |p| p.to_str() == Some("-"));
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
            Some("json") => return "json".into(),
            _ => {}
        }
    }
    "toon".into() // default for stdin
}

// ── Filter execution ───────────────────────────────────────────────────────

/// Check if jaq is available. If not, we fall back to jq.
fn which_jaq() -> bool {
    std::process::Command::new("jaq")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_jaq_filter(value: &Value, filter: &str, output_format: &str) -> anyhow::Result<Value> {
    let json_str = serde_json::to_string(value)?;

    // Try jaq first, fall back to jq
    let engine = if which_jaq() { "jaq" } else { "jq" };

    let mut cmd = Command::new(engine);
    cmd.arg("-c"); // compact output — one JSON value per line
    cmd.arg(filter);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .context(format!(
            "Failed to spawn {engine}. Install jaq (cargo install jaq) or jq (apt install jq)."
        ))?;

    // Write input JSON and close stdin
    {
        let mut stdin = child.stdin.take()
            .context("Failed to open jaq stdin")?;
        stdin.write_all(json_str.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{engine} failed (status {}): {stderr}", output.status);
    }

    let stdout = String::from_utf8(output.stdout)?;

    // Raw mode — print jaq output directly and exit
    if output_format == "raw" {
        if !stdout.is_empty() {
            print!("{stdout}");
        }
        // Return a dummy value so the caller doesn't print anything else
        return Ok(Value::Null);
    }

    let stdout = stdout.trim();

    if stdout.is_empty() {
        return Ok(Value::Null);
    }

    // Parse results — jaq -c outputs one JSON value per line
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.is_empty() {
        return Ok(Value::Null);
    }

    if lines.len() == 1 {
        return Ok(serde_json::from_str(lines[0])?);
    }

    let items: Result<Vec<Value>, _> = lines.iter()
        .map(|l| serde_json::from_str(l))
        .collect();
    Ok(Value::Array(items?))
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
            // raw is handled in run_jaq_filter, but if we get here
            // (e.g., from --head without a filter), just print JSON
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
    let json_bytes = serde_json::to_string(value).map(|s| s.len()).unwrap_or(0);

    let record_count = match value {
        Value::Array(arr) => arr.len(),
        _ => 1,
    };

    // Rough token estimate: 1 token ≈ 4 characters
    let toon_tokens = toon_bytes / 4;
    let json_tokens = json_bytes / 4;

    println!("Records:       {record_count}");
    println!("TOON bytes:    {toon_bytes}");
    println!("JSON bytes:    {json_bytes}");
    if json_bytes > 0 {
        let savings = 100.0 * (1.0 - toon_bytes as f64 / json_bytes as f64);
        println!("Token savings: {savings:.1}% (TOON vs JSON)");
    }
    println!("Est. tokens:   TOON ~{toon_tokens}, JSON ~{json_tokens}");
}
