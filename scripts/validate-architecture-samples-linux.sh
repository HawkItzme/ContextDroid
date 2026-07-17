#!/usr/bin/env bash
set -euo pipefail

sample_root=${1:?usage: validate-architecture-samples-linux.sh <architecture-samples-root>}
contextdroid=${2:-"$PWD/target/release/contextdroid"}
result_dir=${3:-"$PWD/validation-results"}
expected_sha=ee66e1526b84c026615df032c705842b7d2a521f

sample_root=$(cd "$sample_root" && pwd)
contextdroid=$(cd "$(dirname "$contextdroid")" && pwd)/$(basename "$contextdroid")
mkdir -p "$result_dir"
result_dir=$(cd "$result_dir" && pwd)

test "$(git -C "$sample_root" rev-parse HEAD)" = "$expected_sha"
grep -q "Apache License" "$sample_root/LICENSE"
test -x "$contextdroid"

fixture="$sample_root/app/src/main/java/com/example/android/architecture/blueprints/todoapp/ContextDroidValidationFailure.kt"
cleanup() {
  rm -f "$fixture"
}
trap cleanup EXIT HUP INT TERM

cat > "$fixture" <<'KOTLIN'
package com.example.android.architecture.blueprints.todoapp

internal fun contextDroidValidationFailure() = syntheticMissingContextDroidSymbol
KOTLIN

export CONTEXTDROID_RUNS_DIR="$result_dir/runs"
export CONTEXTDROID_RETAIN_SUCCESSES=1

set +e
start=$(date +%s%N)
(cd "$sample_root" && ./gradlew :app:compileDebugKotlin --no-daemon) \
  >"$result_dir/raw.stdout.log" 2>"$result_dir/raw.stderr.log"
raw_exit=$?
raw_duration_ns=$(( $(date +%s%N) - start ))

start=$(date +%s%N)
(cd "$sample_root" && "$contextdroid" --profile contextdroid-safe gradlew \
  :app:compileDebugKotlin --no-daemon) \
  >"$result_dir/contextdroid.stdout.log" 2>"$result_dir/contextdroid.stderr.log"
optimized_exit=$?
optimized_duration_ns=$(( $(date +%s%N) - start ))
set -e

test "$raw_exit" -ne 0
test "$optimized_exit" -eq "$raw_exit"
grep -q "syntheticMissingContextDroidSymbol" "$result_dir/raw.stdout.log" \
  "$result_dir/raw.stderr.log"
grep -q "syntheticMissingContextDroidSymbol" "$result_dir/contextdroid.stdout.log" \
  "$result_dir/contextdroid.stderr.log"

raw_bytes=$(( $(wc -c < "$result_dir/raw.stdout.log") + $(wc -c < "$result_dir/raw.stderr.log") ))
returned_bytes=$(( $(wc -c < "$result_dir/contextdroid.stdout.log") + $(wc -c < "$result_dir/contextdroid.stderr.log") ))
raw_lines=$(( $(wc -l < "$result_dir/raw.stdout.log") + $(wc -l < "$result_dir/raw.stderr.log") ))
returned_lines=$(( $(wc -l < "$result_dir/contextdroid.stdout.log") + $(wc -l < "$result_dir/contextdroid.stderr.log") ))

cat > "$result_dir/result.json" <<JSON
{
  "project": "android/architecture-samples",
  "commit": "$expected_sha",
  "license": "Apache-2.0",
  "workload": ":app:compileDebugKotlin synthetic unresolved reference",
  "raw_exit_code": $raw_exit,
  "contextdroid_exit_code": $optimized_exit,
  "raw_bytes": $raw_bytes,
  "returned_bytes": $returned_bytes,
  "raw_lines": $raw_lines,
  "returned_lines": $returned_lines,
  "raw_tokens_estimate": $(( (raw_bytes + 3) / 4 )),
  "returned_tokens_estimate": $(( (returned_bytes + 3) / 4 )),
  "raw_duration_ms": $(( raw_duration_ns / 1000000 )),
  "contextdroid_duration_ms": $(( optimized_duration_ns / 1000000 )),
  "root_message_preserved": true,
  "exit_code_parity": true
}
JSON

cat "$result_dir/result.json"
