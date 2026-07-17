#!/usr/bin/env bash
set -euo pipefail

root="$PWD"
sample="$root/tests/smoke/android-app"
bin="$root/target/release/contextdroid"
export CONTEXTDROID_RUNS_DIR="$(mktemp -d)"
export FAKE_ADB_ARGS="$(mktemp)"
trap 'rm -rf "$CONTEXTDROID_RUNS_DIR"; rm -f "$FAKE_ADB_ARGS"' EXIT

chmod +x "$root/tests/smoke/fake-adb/adb"
PATH="$root/tests/smoke/fake-adb:$PATH" \
  "$bin" logcat snapshot --mode crash --package com.example.contextdroid > logcat.out
grep -q "java.lang.IllegalStateException: synthetic smoke crash" logcat.out
grep -q -- "-t" "$FAKE_ADB_ARGS"
grep -q -- "-m 20000" "$FAKE_ADB_ARGS"
grep -q -- "-v threadtime" "$FAKE_ADB_ARGS"

cd "$sample"
gradle --no-daemon assembleDebug

cp scenarios/KotlinFailure.kt app/src/main/java/com/example/contextdroid/KotlinFailure.kt
set +e
"$bin" gradlew --no-daemon compileDebugKotlin > kotlin.out 2>&1
status=$?
set -e
test "$status" -ne 0
grep -q "Unresolved reference" kotlin.out
rm app/src/main/java/com/example/contextdroid/KotlinFailure.kt

mkdir -p app/src/main/res/layout
cp scenarios/broken.xml app/src/main/res/layout/broken.xml
set +e
"$bin" gradlew --no-daemon processDebugResources > aapt.out 2>&1
status=$?
set -e
test "$status" -ne 0
grep -Eqi "AAPT|resource" aapt.out
rm app/src/main/res/layout/broken.xml

mkdir -p app/src/test/java/com/example/contextdroid
cp scenarios/FailingTest.kt app/src/test/java/com/example/contextdroid/FailingTest.kt
set +e
"$bin" gradlew --no-daemon testDebugUnitTest > test.out 2>&1
status=$?
set -e
test "$status" -ne 0
grep -Eq "FailingTest|FAILED" test.out

echo "Android success/Kotlin/AAPT/unit-test smoke passed"
