#!/usr/bin/env bash
set -euo pipefail

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
bin="$PWD/target/release/contextdroid"

"$bin" integrations claude preview --root "$root/.claude" | grep -q "contextdroid hook claude"
"$bin" integrations claude install --root "$root/.claude"
"$bin" integrations claude status --root "$root/.claude" | grep -q "installed"
"$bin" integrations claude uninstall --root "$root/.claude"
test ! -e "$root/.claude/settings.json" || ! grep -q "contextdroid hook claude" "$root/.claude/settings.json"

mkdir "$root/repo"
cd "$root/repo"
git init -q
git config user.email contextdroid@example.invalid
git config user.name ContextDroid
printf 'before\n' > tracked.txt
git add tracked.txt
git commit -qm baseline
printf 'after\n' >> tracked.txt
"$bin" git status > status.txt
grep -q " M tracked.txt" status.txt

echo "Claude lifecycle and compact git status smoke passed"
