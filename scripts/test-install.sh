#!/usr/bin/env sh
# Mocked, network-free installer contract tests.

set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
INSTALLER="$ROOT/install.sh"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
FAIL=0

pass() { printf 'PASS: %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; FAIL=1; }

mkdir -p "$TMP/bin" "$TMP/release" "$TMP/package" "$TMP/home" "$TMP/install"
cat > "$TMP/bin/uname" <<'EOF'
#!/usr/bin/env sh
case "${1:-}" in
  -s) printf 'Linux\n' ;;
  -m) printf 'x86_64\n' ;;
  *) printf 'Linux\n' ;;
esac
EOF
chmod +x "$TMP/bin/uname"

make_release() {
  version=$1
  body=${2:-"exit 0"}
  rm -rf "$TMP/package"
  mkdir -p "$TMP/package"
  cat > "$TMP/package/contextdroid" <<EOF
#!/usr/bin/env sh
printf 'contextdroid $version\\n'
$body
EOF
  chmod +x "$TMP/package/contextdroid"
  printf 'license\n' > "$TMP/package/LICENSE"
  printf 'upstream\n' > "$TMP/package/UPSTREAM.md"
  printf 'notices\n' > "$TMP/package/THIRD_PARTY_NOTICES.md"
  tar -czf "$TMP/release/contextdroid-x86_64-unknown-linux-musl.tar.gz" \
    -C "$TMP/package" contextdroid LICENSE UPSTREAM.md THIRD_PARTY_NOTICES.md
  (
    cd "$TMP/release"
    sha256sum contextdroid-x86_64-unknown-linux-musl.tar.gz > SHA256SUMS
  )
}

run_explicit() {
  CONTEXTDROID_VERSION="${1:-}" \
  CONTEXTDROID_RELEASE_BASE="$TMP/release" \
  CONTEXTDROID_INSTALL_DIR="$TMP/install" \
  HOME="$TMP/home" PATH="$TMP/bin:$PATH" \
    sh "$INSTALLER"
}

make_release "1.2.3"
if run_explicit v1.2.3 >"$TMP/explicit.log" 2>&1 &&
  "$TMP/install/contextdroid" --version | grep -Fxq 'contextdroid 1.2.3'; then
  pass "explicit stable pin installs"
else
  sed 's/^/  /' "$TMP/explicit.log" >&2
  fail "explicit stable pin"
fi

if CONTEXTDROID_RELEASE_BASE="$TMP/release" CONTEXTDROID_INSTALL_DIR="$TMP/install" \
  HOME="$TMP/home" PATH="$TMP/bin:$PATH" sh "$INSTALLER" >/dev/null 2>&1; then
  fail "custom base without explicit version was accepted"
else
  pass "custom base requires explicit version"
fi

if run_explicit 'not-a-version' >/dev/null 2>&1; then
  fail "invalid version was accepted"
else
  pass "invalid version rejected"
fi

make_release "1.2.3"
printf '%064d  contextdroid-x86_64-unknown-linux-musl.tar.gz\n' 0 > "$TMP/release/SHA256SUMS"
if run_explicit v1.2.3 >/dev/null 2>&1; then
  fail "checksum mismatch was accepted"
else
  pass "checksum mismatch rejected"
fi

make_release "9.9.9"
if run_explicit v1.2.3 >/dev/null 2>&1; then
  fail "binary version mismatch was accepted"
else
  pass "binary version mismatch rejected"
fi

if command -v python3 >/dev/null 2>&1 && python3 --version >/dev/null 2>&1; then
  python3 - "$TMP/release/contextdroid-x86_64-unknown-linux-musl.tar.gz" <<'PY'
import io
import sys
import tarfile

with tarfile.open(sys.argv[1], "w:gz") as archive:
    info = tarfile.TarInfo("../contextdroid")
    payload = b"unsafe"
    info.size = len(payload)
    archive.addfile(info, io.BytesIO(payload))
PY
  (
    cd "$TMP/release"
    sha256sum contextdroid-x86_64-unknown-linux-musl.tar.gz > SHA256SUMS
  )
  if run_explicit v1.2.3 >/dev/null 2>&1; then
    fail "unsafe archive was accepted"
  else
    pass "unsafe archive rejected before extraction"
  fi
fi

cat > "$TMP/bin/curl" <<'EOF'
#!/usr/bin/env sh
out=
url=
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o|--output) out=$2; shift 2 ;;
    --write-out|--proto|--proto-redir|--max-redirs|--connect-timeout|--max-time)
      shift 2 ;;
    --fail|--silent|--show-error|--location|--tlsv1.2) shift ;;
    https://*) url=$1; shift ;;
    *) shift ;;
  esac
done
case "$url" in
  */releases/latest)
    case "${FAKE_LATEST_MODE:-stable}" in
      stable) printf 'https://github.com/HawkItzme/ContextDroid/releases/tag/v1.2.3' ;;
      unsafe) printf 'https://evil.example/releases/tag/v1.2.3' ;;
      none) exit 22 ;;
    esac ;;
  */releases/download/v1.2.3/*)
    cp "$FAKE_RELEASE/${url##*/}" "$out" ;;
  *) exit 22 ;;
esac
EOF
chmod +x "$TMP/bin/curl"

make_release "1.2.3"
rm -f "$TMP/install/contextdroid"
if FAKE_RELEASE="$TMP/release" CONTEXTDROID_INSTALL_DIR="$TMP/install" \
  HOME="$TMP/home" PATH="$TMP/bin:$PATH" sh "$INSTALLER" >/dev/null 2>&1; then
  pass "latest stable release discovery installs"
else
  fail "latest stable release discovery"
fi
for mode in none unsafe; do
  if FAKE_LATEST_MODE="$mode" FAKE_RELEASE="$TMP/release" \
    CONTEXTDROID_INSTALL_DIR="$TMP/install" HOME="$TMP/home" \
    PATH="$TMP/bin:$PATH" sh "$INSTALLER" >/dev/null 2>&1; then
    fail "latest discovery mode $mode was accepted"
  else
    pass "latest discovery mode $mode rejected"
  fi
done

printf '#!/usr/bin/env sh\nprintf "contextdroid old\\n"\n' > "$TMP/install/contextdroid"
chmod +x "$TMP/install/contextdroid"
make_release "1.2.3" 'count=$(cat "$COUNTER_FILE" 2>/dev/null || printf 0); count=$((count + 1)); printf "%s" "$count" > "$COUNTER_FILE"; [ "$count" -lt 2 ]'
if COUNTER_FILE="$TMP/counter" run_explicit v1.2.3 >/dev/null 2>&1; then
  fail "post-install verification failure was accepted"
elif "$TMP/install/contextdroid" --version | grep -Fxq 'contextdroid old'; then
  pass "failed replacement rolls back previous binary"
else
  fail "failed replacement did not restore previous binary"
fi

[ "$FAIL" -eq 0 ] || exit 1
printf 'All Unix installer tests passed\n'
