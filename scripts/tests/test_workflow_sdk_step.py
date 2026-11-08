"""Runs the workflow's 'Android SDK packages' step against a fake sdkmanager that reproduces the real quirks:
an unknown package gives a warning and exit code 0; a corrupted download gives 'Error on ZipFile unknown archive'."""
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

WORKFLOW = Path(__file__).resolve().parent.parent.parent / ".github/workflows/android.yml"


def sdk_step_script() -> str:
    lines = WORKFLOW.read_text(encoding="utf-8").splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("      - name: Android SDK packages"))
    run = next(i for i in range(start, len(lines)) if lines[i].strip() == "run: |")
    body = []
    for line in lines[run + 1:]:
        if line.strip() and not line.startswith(" " * 10):
            break
        body.append(line[10:] if line.strip() else "")
    return "\n".join(body)


FAKE = r'''#!/usr/bin/env bash
state="$ANDROID_HOME/.installed"; touch "$state"
calls="$ANDROID_HOME/.install_calls"
case "$1" in
  --licenses) head -n 3 > /dev/null; exit 0;;
  --list) for p in $MOCK_KNOWN; do echo "  $p | 1 | pkg | path"; done; exit 0;;
  --list_installed) cat "$state"; exit 0;;
esac
echo x >> "$calls"; n=$(wc -l < "$calls")
if [ "${MOCK_FAIL_FIRST:-0}" -ge "$n" ]; then
  mkdir -p "$ANDROID_HOME/ndk/$MOCK_NDK"            # a half-extracted package, as left by a failed download
  echo "Error on ZipFile unknown archive" >&2
  exit "${MOCK_FAIL_STATUS:-1}"
fi
for p in "$@"; do
  if echo " $MOCK_KNOWN " | grep -qF " $p "; then
    [ "${MOCK_SKIP_RECORD:-}" = "$p" ] || echo "$p | 1 | installed" >> "$state"
    d="${p#*;}"; [ "${p%%;*}" = platforms ] && mkdir -p "$ANDROID_HOME/platforms/$d"
    echo "Installing $p"
  else
    echo "Warning: Failed to find package '$p'" >&2
  fi
done
exit 0
'''
KNOWN = "platforms;android-37.0 build-tools;36.0.0 ndk;28.2.13676358"


def run_step(platform="android-37.0", known=KNOWN, **mock_env):
    tmp = Path(tempfile.mkdtemp())
    sdk = tmp / "sdk"
    (sdk / "cmdline-tools/latest/bin").mkdir(parents=True)
    fake = sdk / "cmdline-tools/latest/bin/sdkmanager"
    fake.write_text(FAKE)
    fake.chmod(0o755)
    env = dict(os.environ, ANDROID_HOME=str(sdk), PLATFORM=platform, BUILD_TOOLS_VERSION="36.0.0",
               NDK_VERSION="28.2.13676358", MOCK_NDK="28.2.13676358", MOCK_KNOWN=known, SDK_RETRY_DELAY="0")
    env.update({k.upper(): str(v) for k, v in mock_env.items()})
    result = subprocess.run(["bash", "-e", "-c", sdk_step_script()], cwd=tmp, env=env,
                            capture_output=True, text=True, timeout=30)
    calls = (sdk / ".install_calls")
    attempts = len(calls.read_text().splitlines()) if calls.exists() else 0
    try:
        return result, attempts, sdk
    finally:
        pass


class SdkStepTests(unittest.TestCase):
    def test_extracted_step_is_the_real_one(self):
        script = sdk_step_script()
        self.assertIn("--list_installed", script)
        self.assertIn("Error on ZipFile", script)

    def test_success_on_the_first_attempt(self):
        result, attempts, _ = run_step()
        self.assertEqual((result.returncode, attempts), (0, 1), result.stdout + result.stderr)
        self.assertIn("SDK packages verified", result.stdout)

    def test_unknown_package_is_permanent_and_not_retried(self):
        result, attempts, _ = run_step(platform="android-37")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(attempts, 1)
        self.assertIn("could not find a requested package", result.stdout)

    def test_corrupted_download_is_retried_and_partial_directories_are_removed(self):
        result, attempts, sdk = run_step(mock_fail_first=1)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(attempts, 2)
        self.assertIn("attempt 1 failed", result.stdout)
        self.assertFalse((sdk / "ndk/28.2.13676358").exists(), "the half-extracted NDK directory was not cleaned up")
        self.assertIn("SDK packages verified", result.stdout)

    def test_two_failures_then_success(self):
        result, attempts, _ = run_step(mock_fail_first=2)
        self.assertEqual((result.returncode, attempts), (0, 3), result.stdout + result.stderr)

    def test_persistent_failure_ends_with_an_error_after_three_attempts(self):
        result, attempts, _ = run_step(mock_fail_first=99)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(attempts, 3)
        self.assertIn("failed 3 times", result.stdout)

    def test_zip_error_is_retried_even_when_the_exit_status_is_zero(self):
        result, attempts, _ = run_step(mock_fail_first=1, mock_fail_status=0)
        self.assertEqual((result.returncode, attempts), (0, 2), result.stdout + result.stderr)

    def test_package_that_is_not_in_the_installed_list_fails_verification(self):
        result, attempts, _ = run_step(mock_skip_record="ndk;28.2.13676358")
        self.assertEqual(result.returncode, 1)
        self.assertIn("is not installed", result.stdout)

    def test_unknown_ndk_is_permanent(self):
        result, attempts, _ = run_step(known="platforms;android-37.0 build-tools;36.0.0")
        self.assertEqual((result.returncode, attempts), (1, 1))


if __name__ == "__main__":
    unittest.main()
