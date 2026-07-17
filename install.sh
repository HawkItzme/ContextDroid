#!/usr/bin/env sh
# ContextDroid checksum-verifying release installer.

set -eu

REPO="HawkItzme/ContextDroid"
BINARY="contextdroid"
INSTALL_DIR="${CONTEXTDROID_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${CONTEXTDROID_VERSION:-v0.1.0-alpha.1}"

fail() { printf '%s\n' "ContextDroid install error: $1" >&2; exit 1; }

case "$(uname -s):$(uname -m)" in
  Linux:x86_64|Linux:amd64)
    asset="contextdroid-x86_64-unknown-linux-musl.tar.gz" ;;
  Linux:aarch64|Linux:arm64)
    asset="contextdroid-aarch64-unknown-linux-gnu.tar.gz" ;;
  Darwin:x86_64|Darwin:amd64)
    asset="contextdroid-x86_64-apple-darwin.tar.gz" ;;
  Darwin:arm64|Darwin:aarch64)
    asset="contextdroid-aarch64-apple-darwin.tar.gz" ;;
  *) fail "unsupported operating system or architecture" ;;
esac

printf '%s' "$VERSION" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$' ||
  fail "invalid CONTEXTDROID_VERSION"

base="${CONTEXTDROID_RELEASE_BASE:-https://github.com/$REPO/releases/download/$VERSION}"
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT HUP INT TERM

curl -fsSL "$base/$asset" -o "$temp/$asset" || fail "asset download failed"
curl -fsSL "$base/SHA256SUMS" -o "$temp/SHA256SUMS" || fail "checksum download failed"
expected=$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "$temp/SHA256SUMS")
[ -n "$expected" ] || fail "asset checksum is missing"
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$temp/$asset" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$temp/$asset" | awk '{print $1}')
else
  fail "no SHA-256 tool is available"
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch"
if tar -tzf "$temp/$asset" | grep -qE '^/|(^|/)\.\.(/|$)'; then
  fail "unsafe archive path"
fi
tar -xzf "$temp/$asset" -C "$temp"
[ -f "$temp/$BINARY" ] || fail "archive does not contain contextdroid"
expected_version=${VERSION#v}
"$temp/$BINARY" --version | grep -F "$expected_version" >/dev/null ||
  fail "downloaded binary version does not match $VERSION"
mkdir -p "$INSTALL_DIR"
staged="$INSTALL_DIR/.contextdroid.new.$$"
install -m 0755 "$temp/$BINARY" "$staged"
mv -f "$staged" "$INSTALL_DIR/$BINARY"
"$INSTALL_DIR/$BINARY" --version
