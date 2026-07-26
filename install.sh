#!/usr/bin/env sh
# ContextDroid checksum-verifying release installer.

set -eu

REPO="HawkItzme/ContextDroid"
BINARY="contextdroid"
LATEST_URL="https://github.com/$REPO/releases/latest"
INSTALL_DIR="${CONTEXTDROID_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${CONTEXTDROID_VERSION:-}"
CUSTOM_BASE="${CONTEXTDROID_RELEASE_BASE:-}"

fail() { printf '%s\n' "ContextDroid install error: $1" >&2; exit 1; }

validate_version() {
  printf '%s' "$1" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$' ||
    fail "invalid CONTEXTDROID_VERSION"
}

download_https() {
  curl --fail --silent --show-error --location \
    --proto '=https' --proto-redir '=https' --tlsv1.2 \
    --max-redirs 5 --connect-timeout 10 --max-time 180 \
    "$1" -o "$2"
}

resolve_latest_stable() {
  effective=$(
    curl --fail --silent --show-error --location \
      --proto '=https' --proto-redir '=https' --tlsv1.2 \
      --max-redirs 5 --connect-timeout 10 --max-time 30 \
      --output /dev/null --write-out '%{url_effective}' "$LATEST_URL"
  ) || fail "no stable ContextDroid release is available"
  prefix="https://github.com/$REPO/releases/tag/"
  case "$effective" in
    "$prefix"v[0-9]*.[0-9]*.[0-9]*) ;;
    *) fail "latest release redirected to an unexpected URL" ;;
  esac
  resolved=${effective#"$prefix"}
  printf '%s' "$resolved" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' ||
    fail "latest release is not a stable semantic version"
  printf '%s\n' "$resolved"
}

copy_release_file() {
  copy_name=$1
  copy_destination=$2
  case "$RELEASE_BASE" in
    file://*)
      cp "${RELEASE_BASE#file://}/$copy_name" "$copy_destination" ;;
    http://*)
      fail "release downloads must use HTTPS" ;;
    https://*)
      download_https "$RELEASE_BASE/$copy_name" "$copy_destination" ;;
    *)
      [ -d "$RELEASE_BASE" ] || fail "custom release base is not a directory"
      cp "$RELEASE_BASE/$copy_name" "$copy_destination" ;;
  esac
}

if [ -n "$CUSTOM_BASE" ] && [ -z "$VERSION" ]; then
  fail "custom release base requires CONTEXTDROID_VERSION"
fi
if [ -n "$VERSION" ]; then
  validate_version "$VERSION"
else
  VERSION=$(resolve_latest_stable)
fi

RELEASE_BASE=${CUSTOM_BASE:-"https://github.com/$REPO/releases/download/$VERSION"}
RELEASE_BASE=${RELEASE_BASE%/}

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

temp=$(mktemp -d "${TMPDIR:-/tmp}/contextdroid-install.XXXXXX")
chmod 700 "$temp"
backup=""
install_destination="$INSTALL_DIR/$BINARY"
committed=0
cleanup() {
  if [ "$committed" -eq 0 ] && [ -n "$backup" ] && [ -f "$backup" ]; then
    rm -f "$install_destination"
    mv "$backup" "$install_destination"
  fi
  rm -rf "$temp"
}
trap cleanup EXIT HUP INT TERM

copy_release_file "$asset" "$temp/$asset" || fail "asset download failed"
copy_release_file "SHA256SUMS" "$temp/SHA256SUMS" || fail "checksum download failed"

expected=$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "$temp/SHA256SUMS")
printf '%s' "$expected" | grep -Eq '^[0-9A-Fa-f]{64}$' ||
  fail "asset checksum is missing or invalid"
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$temp/$asset" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$temp/$asset" | awk '{print $1}')
else
  fail "no SHA-256 tool is available"
fi
[ "$(printf '%s' "$expected" | tr 'A-F' 'a-f')" = "$(printf '%s' "$actual" | tr 'A-F' 'a-f')" ] ||
  fail "checksum mismatch"

tar -tzf "$temp/$asset" > "$temp/archive-entries"
[ "$(grep -c '^contextdroid$' "$temp/archive-entries")" -eq 1 ] ||
  fail "archive does not contain exactly one contextdroid binary"
while IFS= read -r entry; do
  case "$entry" in
    contextdroid|LICENSE|UPSTREAM.md|THIRD_PARTY_NOTICES.md) ;;
    *) fail "unsafe archive path or unexpected archive entry" ;;
  esac
done < "$temp/archive-entries"
tar -xzf "$temp/$asset" -C "$temp" contextdroid
[ -f "$temp/$BINARY" ] && [ ! -L "$temp/$BINARY" ] ||
  fail "archive does not contain a regular contextdroid binary"
chmod 0755 "$temp/$BINARY"

expected_version=${VERSION#v}
reported=$("$temp/$BINARY" --version) || fail "downloaded binary could not be executed"
printf '%s\n' "$reported" | grep -Eq "^contextdroid ${expected_version}([[:space:]]|$)" ||
  fail "downloaded binary version does not match $VERSION"

mkdir -p "$INSTALL_DIR"
staged="$INSTALL_DIR/.contextdroid.new.$$"
install -m 0755 "$temp/$BINARY" "$staged"
if [ -e "$install_destination" ]; then
  backup="$INSTALL_DIR/.contextdroid.backup.$$"
  mv "$install_destination" "$backup"
fi
if ! mv "$staged" "$install_destination"; then
  fail "failed to replace ContextDroid binary"
fi
if ! "$install_destination" --version >/dev/null; then
  fail "installed ContextDroid binary failed verification"
fi
committed=1
[ -z "$backup" ] || rm -f "$backup"
"$install_destination" --version
printf '%s\n' "ContextDroid installed to $install_destination"
printf '%s\n' "Next: contextdroid setup detect"
