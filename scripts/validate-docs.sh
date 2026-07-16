#!/usr/bin/env bash
set -euo pipefail

required=(
  README.md UPSTREAM.md THIRD_PARTY_NOTICES.md CHANGELOG.md CLAUDE.md
  docs/ARCHITECTURE.md docs/CONTEXTDROID_PRODUCT_SPEC.md docs/SAFETY_CONTRACT.md
  docs/FILTER_MATRIX.md docs/BENCHMARKS.md docs/INTEGRATIONS.md
  docs/MIGRATION.md docs/RELEASE_CHECKLIST.md .agent/EXEC_PLAN.md
)

for file in "${required[@]}"; do
  test -s "$file" || { echo "missing required documentation: $file" >&2; exit 1; }
done

grep -q "independently maintained" README.md
grep -q "v0.43.0" UPSTREAM.md
grep -q "failure" docs/SAFETY_CONTRACT.md
grep -q "snapshot" docs/FILTER_MATRIX.md
grep -q "migrate rtk" docs/MIGRATION.md

if grep -q "prefer .*rtk" CLAUDE.md; then
  echo "CLAUDE.md must not tell contributors to compress repository commands" >&2
  exit 1
fi

echo "ContextDroid documentation contract passed"
