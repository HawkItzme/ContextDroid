#!/usr/bin/env bash
set -euo pipefail

required=(
  README.md UPSTREAM.md THIRD_PARTY_NOTICES.md CHANGELOG.md CONTRIBUTING.md
  docs/ARCHITECTURE.md docs/SAFETY_CONTRACT.md
  docs/FILTER_MATRIX.md docs/BENCHMARKS.md docs/INTEGRATIONS.md
  docs/MIGRATION.md docs/RELEASE_CHECKLIST.md docs/contributing/TESTING.md
)

for file in "${required[@]}"; do
  test -s "$file" || { echo "missing required documentation: $file" >&2; exit 1; }
done

grep -q "independently maintained" README.md
grep -q "v0.43.0" UPSTREAM.md
grep -q "failure" docs/SAFETY_CONTRACT.md
grep -q "snapshot" docs/FILTER_MATRIX.md
grep -q "migrate rtk" docs/MIGRATION.md
bash scripts/check-stale-brand.sh

for internal in .agent .agents .claude .codex .rtk AGENTS.md CLAUDE.md \
  .github/copilot-instructions.md .github/hooks/rtk-rewrite.json \
  docs/PILOT.md docs/validation/INTERNAL_ANDROID_PROJECT.md \
  docs/maintainers/MAINTAINERS_APPLY.md; do
  if test -f "$internal" || { test -d "$internal" && find "$internal" -type f -print -quit | grep -q .; }; then
    echo "internal development artifact must not be tracked: $internal" >&2
    exit 1
  fi
done

echo "ContextDroid documentation contract passed"
