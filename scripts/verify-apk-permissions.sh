#!/usr/bin/env bash
# Verifies the permissions of a BUILT apk. Exits 0 ONLY after it has read the apk's manifest
# with aapt2 and confirmed that android.permission.INTERNET is not requested.
# Every other outcome (aapt2 missing, apk missing/unreadable, unexpected output) is a failure.
#
# Needs aapt2 from Android SDK build-tools, on PATH or via AAPT2=/path/to/aapt2.
# Usage: scripts/verify-apk-permissions.sh app/build/outputs/apk/debug/app-debug.apk
set -euo pipefail

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

apk="${1:-}"
[ -n "$apk" ] || fail "usage: $0 path/to/app.apk"
[ -f "$apk" ] || fail "apk not found: $apk"
[ -r "$apk" ] || fail "apk is not readable: $apk"

aapt2="${AAPT2:-aapt2}"
if [[ "$aapt2" == */* ]]; then
  [ -x "$aapt2" ] || fail "aapt2 not executable: $aapt2 (set AAPT2 to build-tools/<version>/aapt2)"
else
  command -v "$aapt2" > /dev/null 2>&1 || fail "aapt2 not found on PATH (set AAPT2 to build-tools/<version>/aapt2)"
fi

# 1) badging proves the apk and its manifest are readable and gives the package name.
badging_status=0
badging="$("$aapt2" dump badging "$apk" 2>&1)" || badging_status=$?
[ "$badging_status" -eq 0 ] || fail "aapt2 dump badging failed (exit $badging_status) for $apk:
$badging"
package_line="$(printf '%s\n' "$badging" | grep -m1 "^package: name=" || true)"
[ -n "$package_line" ] || fail "aapt2 output has no 'package: name=' line; cannot confirm the manifest was read:
$badging"

# 2) permission list.
perms_status=0
perms="$("$aapt2" dump permissions "$apk" 2>&1)" || perms_status=$?
[ "$perms_status" -eq 0 ] || fail "aapt2 dump permissions failed (exit $perms_status) for $apk:
$perms"

echo "apk:      $apk"
echo "$package_line"
echo "--- permissions reported by aapt2 dump permissions"
if [ -n "$perms" ]; then printf '%s\n' "$perms"; else echo "(none)"; fi

if printf '%s\n%s\n' "$badging" "$perms" | grep -Eq "android\.permission\.INTERNET"; then
  fail "android.permission.INTERNET is requested by $apk"
fi
echo "OK: manifest of $apk was read and android.permission.INTERNET is not requested"
