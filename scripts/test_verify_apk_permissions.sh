#!/usr/bin/env bash
# Tests the LOGIC of verify-apk-permissions.sh against a fake aapt2. It does not test real aapt2
# output formats; the CI job runs the script against the real apk with the real aapt2.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
script="$here/verify-apk-permissions.sh"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
apk="$tmp/app.apk"; echo dummy > "$apk"
pass=0; failed=0

expect() { # name expected_exit actual_exit
  if [ "$2" -eq "$3" ]; then pass=$((pass+1)); echo "ok   - $1"; else failed=$((failed+1)); echo "FAIL - $1 (expected exit $2, got $3)"; fi
}
mkfake() { # body of fake aapt2 (receives: dump <what> <apk>)
  printf '#!/usr/bin/env bash\n%s\n' "$1" > "$tmp/aapt2"; chmod +x "$tmp/aapt2"
}

AAPT2="$tmp/no-such-aapt2" "$script" "$apk" > /dev/null 2>&1; expect "missing aapt2 fails" 1 $?
AAPT2="" PATH="/nonexistent" "$BASH" "$script" "$apk" > /dev/null 2>&1; expect "aapt2 not on PATH fails" 1 $?
"$script" > /dev/null 2>&1; expect "no argument fails" 1 $?
AAPT2="$tmp/aapt2" "$script" "$tmp/missing.apk" > /dev/null 2>&1; expect "missing apk fails" 1 $?

mkfake 'echo "boom" >&2; exit 1'
AAPT2="$tmp/aapt2" "$script" "$apk" > /dev/null 2>&1; expect "aapt2 failing (unreadable apk) fails" 1 $?

mkfake 'exit 0'
AAPT2="$tmp/aapt2" "$script" "$apk" > /dev/null 2>&1; expect "empty aapt2 output cannot confirm, fails" 1 $?

mkfake 'case "$2" in badging) echo "package: name='"'"'x.y'"'"' versionCode='"'"'1'"'"'";; permissions) echo "package: x.y"; echo "uses-permission: name='"'"'android.permission.INTERNET'"'"'";; esac'
AAPT2="$tmp/aapt2" "$script" "$apk" > /dev/null 2>&1; expect "INTERNET in permissions fails" 1 $?

mkfake 'case "$2" in badging) echo "package: name='"'"'x.y'"'"'"; echo "uses-permission: name='"'"'android.permission.INTERNET'"'"'";; permissions) echo "package: x.y";; esac'
AAPT2="$tmp/aapt2" "$script" "$apk" > /dev/null 2>&1; expect "INTERNET only in badging fails" 1 $?

mkfake 'case "$2" in badging) echo "package: name='"'"'x.y'"'"'";; permissions) echo "package: x.y"; echo "uses-permission: name='"'"'android.permission.ACCESS_NETWORK_STATE'"'"'";; esac'
AAPT2="$tmp/aapt2" "$script" "$apk" > /dev/null 2>&1; expect "other permissions pass" 0 $?

mkfake 'case "$2" in badging) echo "package: name='"'"'x.y'"'"'";; permissions) echo "package: x.y";; esac'
AAPT2="$tmp/aapt2" "$script" "$apk" > /dev/null 2>&1; expect "no permissions passes" 0 $?

echo "passed=$pass failed=$failed"
[ "$failed" -eq 0 ]
