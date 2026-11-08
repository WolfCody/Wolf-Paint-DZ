"""Runs the bootstrap step 'Cargo.lock matches the workspace crates' with a fake `cargo` and the real
scripts/check_lock_change.py: a stale lock may be refreshed only for workspace crates."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
WORKFLOW = ROOT / ".github/workflows/android.yml"

CIO = 'source = "registry+https://github.com/rust-lang/crates.io-index"'
BASE = f'version = 4\n\n[[package]]\nname = "core-model"\nversion = "0.1.0"\n\n[[package]]\nname = "jni"\nversion = "0.21.1"\n{CIO}\nchecksum = "aaa"\n'
WITH_RASTER = BASE + '\n[[package]]\nname = "core-raster"\nversion = "0.1.0"\n'
WITH_LOG = BASE + f'\n[[package]]\nname = "log"\nversion = "0.4.22"\n{CIO}\nchecksum = "bbb"\n'


def step_script() -> str:
    lines = WORKFLOW.read_text(encoding="utf-8").splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("      - name: Cargo.lock matches the workspace crates"))
    run = next(i for i in range(start, len(lines)) if lines[i].strip() == "run: |")
    body = []
    for line in lines[run + 1:]:
        if line.strip() and not line.startswith(" " * 10):
            break
        body.append(line[10:] if line.strip() else "")
    return "\n".join(body)


FAKE_CARGO = r'''#!/usr/bin/env bash
# fake cargo: `metadata --locked` succeeds only when Cargo.lock equals $MOCK_GOOD_LOCK; `update --workspace` writes $MOCK_UPDATED_LOCK
case "$1" in
  metadata)
    if cmp -s Cargo.lock "$MOCK_GOOD_LOCK"; then exit 0; fi
    echo "error: the lock file needs to be updated but --locked was passed to prevent this" >&2; exit 101;;
  update)
    [ "$2" = "--workspace" ] || { echo "unexpected: cargo update $*" >&2; exit 2; }
    cp "$MOCK_UPDATED_LOCK" Cargo.lock; echo updated >> "$MOCK_LOG"; exit 0;;
esac
echo "fake cargo: unsupported $*" >&2; exit 2
'''


def run_step(committed: str, updated: str, good: str):
    tmp = Path(tempfile.mkdtemp())
    work, bin_dir, runner_temp = tmp / "repo", tmp / "bin", tmp / "runner"
    for d in (work / "scripts", bin_dir, runner_temp):
        d.mkdir(parents=True)
    (work / "scripts/check_lock_change.py").write_text((ROOT / "scripts/check_lock_change.py").read_text())
    (work / "Cargo.lock").write_text(committed)
    (tmp / "good.lock").write_text(good)
    (tmp / "updated.lock").write_text(updated)
    cargo = bin_dir / "cargo"
    cargo.write_text(FAKE_CARGO)
    cargo.chmod(0o755)
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}", RUNNER_TEMP=str(runner_temp),
               MOCK_GOOD_LOCK=str(tmp / "good.lock"), MOCK_UPDATED_LOCK=str(tmp / "updated.lock"),
               MOCK_LOG=str(tmp / "log"))
    result = subprocess.run(["bash", "-e", "-c", step_script()], cwd=work, env=env, capture_output=True, text=True, timeout=30)
    updates = (tmp / "log").read_text().count("updated") if (tmp / "log").exists() else 0
    return result, updates, (work / "Cargo.lock").read_text()


class LockStepTests(unittest.TestCase):
    def test_extracted_step_is_the_real_one(self):
        self.assertIn("cargo update --workspace", step_script())
        self.assertIn("scripts/check_lock_change.py", step_script())

    def test_lock_in_sync_is_left_alone(self):
        result, updates, lock = run_step(BASE, WITH_RASTER, good=BASE)
        self.assertEqual((result.returncode, updates, lock), (0, 0, BASE), result.stdout + result.stderr)
        self.assertIn("in sync", result.stdout)

    def test_stale_lock_is_refreshed_for_a_new_workspace_crate(self):
        result, updates, lock = run_step(BASE, WITH_RASTER, good=WITH_RASTER)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((updates, lock), (1, WITH_RASTER))
        self.assertIn("::warning::Cargo.lock is out of date", result.stdout)
        self.assertIn("workspace crate added or changed: core-raster", result.stdout)

    def test_update_that_touches_a_third_party_package_fails_the_step(self):
        result, updates, _ = run_step(BASE, WITH_LOG, good=WITH_LOG)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(updates, 1)
        self.assertIn("third-party package added", result.stdout)

    def test_lock_that_is_still_stale_after_the_update_fails(self):
        result, _, _ = run_step(BASE, WITH_RASTER, good="version = 4\n# something else\n")
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
