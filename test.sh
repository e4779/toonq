#!/bin/bash
set -e
BIN=./target/release/toonq
DATA=/home/e41q/dev/invest-research/data/raw/gldrub.toon
TMP=/tmp/toonq_test
rm -rf $TMP && mkdir -p $TMP

ok() { echo "  ✓"; }

echo "1. --count"
test "$($BIN --count $DATA 2>/dev/null)" = "1480" && ok

echo "2. --head"
$BIN --head 1 $DATA 2>/dev/null | grep -q "2014-01-09" && ok

echo "3. --tail"
$BIN --tail 1 $DATA 2>/dev/null | grep -q "2026-06-02" && ok

echo "4. --schema"
$BIN --schema $DATA 2>/dev/null | grep -q "date: string" && ok

echo "5. filter (no matches → null)"
test "$($BIN -f '.[] | select(.close > 99999)' $DATA 2>/dev/null)" = "null" && ok

echo "6. filter (with matches)"
$BIN -f '.[] | select(.close > 10000) | {date}' $DATA 2>/dev/null | grep -q "2025-10-10" && ok

echo "7. --stats"
$BIN --stats $DATA 2>/dev/null | grep -q "Token savings" && ok

echo "8. json roundtrip"
$BIN --to json $DATA > $TMP/test.json 2>/dev/null
test "$($BIN --from json --count $TMP/test.json 2>/dev/null)" = "1480" && ok

echo "9. stdin pipe"
test "$(cat $DATA | $BIN --count 2>/dev/null)" = "1480" && ok

echo "10. raw output"
$BIN -f '.[0].date' --to raw $DATA 2>/dev/null | grep -q "2014-01-09" && ok

echo "11. sort + slice"
$BIN -f 'sort_by(-.close) | .[0].date' --to raw $DATA 2>/dev/null | grep -q "2026-01-29" && ok

echo "12. --argjson (numeric variable)"
test "$($BIN -f '.[0].close > $threshold' --argjson threshold 1000 $DATA 2>/dev/null)" = "true" && ok

echo "13. --arg (string variable)"
test "$($BIN -f '.[0].date as $d | $d' --arg suffix "x" -r $DATA 2>/dev/null)" = "2014-01-09" && ok

echo "14. -n (null input / generate)"
test "$($BIN -n -f '[range(3)]' --to json 2>/dev/null | tr -d ' \n')" = '[0,1,2]' && ok

echo "15. -r (raw output, no quotes)"
$BIN -f '.[0].date' -r $DATA 2>/dev/null | grep -q '^2014-01-09$' && ok

echo "16. -r (non-string stays compact JSON)"
test "$($BIN -f '.[0].close' -r $DATA 2>/dev/null)" = "1310" && ok

echo ""
echo "✓ All 16 tests passed"
