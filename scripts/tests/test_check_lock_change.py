import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import check_lock_change as c  # noqa: E402

CIO = 'source = "registry+https://github.com/rust-lang/crates.io-index"'


def lock(*packages, version=4):
    out = [f"version = {version}", ""]
    for p in packages:
        out.append("[[package]]")
        out.append(f'name = "{p["name"]}"')
        out.append(f'version = "{p.get("version", "0.1.0")}"')
        if p.get("registry"):
            out.append(CIO)
            out.append(f'checksum = "{p.get("checksum", "abc")}"')
        if p.get("deps"):
            out.append("dependencies = [" + ", ".join(f'"{d}"' for d in p["deps"]) + "]")
        out.append("")
    return "\n".join(out)


JNI = {"name": "jni", "version": "0.21.1", "registry": True, "checksum": "aaa"}
LOG = {"name": "log", "version": "0.4.22", "registry": True, "checksum": "bbb"}
MODEL = {"name": "core-model"}
FFI = {"name": "core-ffi", "deps": ["core-model", "jni"]}
RASTER = {"name": "core-raster"}


def compare(old_text, new_text):
    import tomllib
    return c.compare(tomllib.loads(old_text), tomllib.loads(new_text))


class LockChangeTests(unittest.TestCase):
    def test_new_workspace_crate_is_allowed(self):
        old = lock(JNI, LOG, MODEL, FFI)
        new = lock(JNI, LOG, MODEL, RASTER, FFI)
        errors, notes = compare(old, new)
        self.assertEqual(errors, [])
        self.assertEqual(notes, ["workspace crate added or changed: core-raster 0.1.0"])

    def test_no_change_is_fine(self):
        text = lock(JNI, MODEL)
        self.assertEqual(compare(text, text), ([], []))

    def test_workspace_crate_removed_or_version_bumped_is_allowed(self):
        errors, notes = compare(lock(JNI, MODEL, RASTER), lock(JNI, MODEL))
        self.assertEqual((errors, len(notes)), ([], 1))
        errors, notes = compare(lock(JNI, {"name": "core-model", "version": "0.1.0"}),
                                lock(JNI, {"name": "core-model", "version": "0.2.0"}))
        self.assertEqual((errors, len(notes)), ([], 2))

    def test_new_third_party_crate_is_rejected(self):
        errors, _ = compare(lock(JNI, MODEL), lock(JNI, LOG, MODEL))
        self.assertEqual(len(errors), 1)
        self.assertIn("added", errors[0])
        self.assertIn("log 0.4.22", errors[0])

    def test_removed_third_party_crate_is_rejected(self):
        errors, _ = compare(lock(JNI, LOG, MODEL), lock(JNI, MODEL))
        self.assertTrue(any("removed" in e and "log" in e for e in errors), errors)

    def test_version_bump_of_a_third_party_crate_is_rejected(self):
        bumped = dict(JNI, version="0.21.2")
        errors, _ = compare(lock(JNI, MODEL), lock(bumped, MODEL))
        self.assertEqual(len(errors), 2)  # the old entry is gone, a new one appeared

    def test_checksum_change_is_rejected(self):
        changed = dict(JNI, checksum="zzz")
        errors, _ = compare(lock(JNI, MODEL), lock(changed, MODEL))
        self.assertTrue(errors)

    def test_lock_format_version_change_is_rejected(self):
        errors, _ = compare(lock(JNI, MODEL, version=4), lock(JNI, MODEL, version=5))
        self.assertTrue(any("format version" in e for e in errors), errors)

    def test_main_reports_via_exit_code(self):
        d = Path(tempfile.mkdtemp())
        (d / "old.lock").write_text(lock(JNI, MODEL))
        (d / "ok.lock").write_text(lock(JNI, MODEL, RASTER))
        (d / "bad.lock").write_text(lock(JNI, LOG, MODEL))
        self.assertEqual(c.main([str(d / "old.lock"), str(d / "ok.lock")]), 0)
        self.assertEqual(c.main([str(d / "old.lock"), str(d / "bad.lock")]), 1)
        self.assertEqual(c.main([str(d / "old.lock")]), 2)

    def test_unreadable_or_corrupt_file_is_a_clear_error(self):
        d = Path(tempfile.mkdtemp())
        (d / "ok.lock").write_text(lock(JNI, MODEL))
        (d / "broken.lock").write_text("this is [not toml")
        with self.assertRaises(SystemExit) as caught:
            c.main([str(d / "ok.lock"), str(d / "broken.lock")])
        self.assertIn("cannot read", str(caught.exception))
        with self.assertRaises(SystemExit):
            c.main([str(d / "ok.lock"), str(d / "missing.lock")])


if __name__ == "__main__":
    unittest.main()
