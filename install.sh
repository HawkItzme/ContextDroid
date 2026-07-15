#!/usr/bin/env sh
# ContextDroid release installer. Releases are not published during development.

set -eu

REPO="HawkItzme/ContextDroid"
BINARY="contextdroid"
INSTALL_DIR="${CONTEXTDROID_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${CONTEXTDROID_VERSION:-}"

fail() { printf '%s\n' "ContextDroid install error: $1" >&2; exit 1; }

case "$(uname -s)" in
  Linux) os="unknown-linux-gnu" ;;
  Darwin) os="apple-darwin" ;;
  *) fail "unsupported operating system" ;;
esac

case "$(uname -m)" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *) fail "unsupported architecture" ;;
esac

if [ -z "$VERSION" ]; then
  VERSION=$(curl -fsSI "https://github.com/$REPO/releases/latest" |
    sed -n 's|.*location: .*/tag/\([^[:space:]]*\).*|\1|Ip' | tr -d '\r')
fi
[ -n "$VERSION" ] || fail "no release found; set CONTEXTDROID_VERSION to a published tag"

asset="$BINARY-$arch-$os.tar.gz"
base="https://github.com/$REPO/releases/download/$VERSION"
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT HUP INT TERM

curl -fsSL "$base/$asset" -o "$temp/$asset" || fail "asset download failed"
curl -fsSL "$base/checksums.txt" -o "$temp/checksums.txt" || fail "checksum download failed"
expected=$(awk -v name="$asset" '$2 == name { print $1 }' "$temp/checksums.txt")
[ -n "$expected" ] || fail "asset checksum is missing"
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$temp/$asset" | awk '{print $1}')
else
  actual=$(shasum -a 256 "$temp/$asset" | awk '{print $1}')
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch"
tar -tzf "$temp/$asset" | grep -qE '^/|(^|/)\.\.(/|$)' && fail "unsafe archive path"
tar -xzf "$temp/$asset" -C "$temp"
[ -f "$temp/$BINARY" ] || fail "archive does not contain contextdroid"
mkdir -p "$INSTALL_DIR"
install -m 0755 "$temp/$BINARY" "$INSTALL_DIR/$BINARY"
"$INSTALL_DIR/$BINARY" --version
