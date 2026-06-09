# Recipes: real-world toonq workflows

## Chat logs (LLM sessions, dialogues)

```bash
# Quick overview
toonq --slurp --schema session.jsonl          # fields: type, role, text...
toonq --slurp --count session.jsonl           # how many messages?
toonq --slurp -f 'group_by(.message.role) | map({role: .[0].message.role, count: length})' session.jsonl

# Extract all messages of one role — ONE call, not 31
toonq --slurp --extract text session.jsonl    # all text fields
toonq --slurp --truncate 80 --extract text session.jsonl  # truncated for scanning

# Filter by role (user messages only)
toonq --slurp -f 'map(select(.message.role == "user")) | map(.message.content[0].text)' session.jsonl

# Count tool calls
toonq --slurp -f '
  map(select(.type=="message" and .message.role=="assistant")) |
  map(.message.content | map(select(.type=="toolCall")) | length) |
  add
' --to raw session.jsonl

# List all bash commands
toonq --slurp -f '
  map(select(.type=="message" and .message.role=="assistant")) |
  map(.message.content[]? | select(.type=="toolCall" and .name=="bash")) |
  map(.arguments.command)
' --to raw session.jsonl
```

## JSONL files

```bash
# Auto-detect (when JSON parse fails, tries JSONL)
toonq --from json --count data.jsonl

# Explicit
toonq --slurp --count data.jsonl

# Pipeline: convert JSONL to TOON
toonq --slurp data.jsonl > data.toon
```

## Financial data (invest-research)

```bash
# Top 3 highest closes
toonq -f 'sort_by(-.close) | .[0:3] | .[] | {date, close}' data.toon

# Filter by value
toonq -f '.[] | select(.close > 10000) | {date, close}' data.toon

# Extract all close prices
toonq --extract close data.toon

# Stats
toonq --stats data.toon
```

## Format conversion

```bash
toonq --to json data.toon           # TOON → JSON
toonq --from json data.json         # JSON → TOON
toonq --from json --to toon data.json  # explicit roundtrip

# Pipeline with jq
toonq --to json data.toon | jq '. | length'
toonq --extract close --to raw data.toon | jq 'add/length'  # average
```

## Inspection

```bash
toonq --head 5 data.toon            # first 5 records
toonq --tail 3 data.toon            # last 3
toonq --count data.toon             # record count
toonq --schema data.toon            # field names + types
toonq --stats data.toon             # TOON vs JSON comparison

# With truncation for long text fields
toonq --truncate 40 --head 3 data.toon
```

## Pipelines

```bash
# Chain filters
toonq -f '.[] | select(.close > 100)' data.toon | toonq --head 3

# Slurp → filter → extract
toonq --slurp -f 'map(select(.type=="message"))' session.jsonl | toonq --extract id --to raw

# JSONL → TOON → filter
toonq --slurp session.jsonl | toonq -f 'group_by(.type) | map({type: .[0].type, count: length})'
```
