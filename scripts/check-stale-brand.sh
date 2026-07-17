#!/usr/bin/env bash
set -euo pipefail

# RTK references are valid only for explicit attribution, migration, compatibility, and
# preserved history. These signatures identify inherited product/contact/install claims that
# must never return to ContextDroid-facing files.
patterns=(
  'security@rtk-ai\.app'
  'contact@rtk-ai\.app'
  'https?://www\.rtk-ai\.app'
  'raw\.githubusercontent\.com/rtk-ai/rtk'
  'cargo install --git https://github\.com/rtk-ai/rtk'
  'Contributing to rtk'
  '/rtk-pr-security'
  'collects anonymous, aggregate usage metrics by default'
)

failed=0
for pattern in "${patterns[@]}"; do
  matches=$(git grep -nEI "$pattern" -- \
    ':!CHANGELOG.md' \
    ':!UPSTREAM.md' \
    ':!THIRD_PARTY_NOTICES.md' \
    ':!scripts/check-stale-brand.sh' || true)
  if [[ -n "$matches" ]]; then
    printf 'stale product identity matched /%s/:\n%s\n' "$pattern" "$matches" >&2
    failed=1
  fi
done

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

echo "ContextDroid stale-brand contract passed"
