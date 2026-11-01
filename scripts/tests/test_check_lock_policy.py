import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import check_lock_policy as p  # noqa: E402

DENY = (Path(__file__).resolve().parent.parent.parent / "deny.toml").read_text(encoding="utf-8")
CIO = 'source = "registry+https://github.com/rust-lang/crates.io-index"'

OK_LOCK = f"""
version = 4
[[package]]
name = "core-model"
version = "0.1.0"
[[package]]
name = "jni"
version = "0.21.1"
{CIO}
checksum = "abc"
"""


class LockPolicyTests(unittest.TestCase):
    def test_clean_lock_passes(self):
        errors, count = p.check(OK_LOCK, DENY)
        self.assertEqual((errors, count), ([], 2))

    def test_banned_crate_is_found(self):
        lock = OK_LOCK + f'[[package]]\nname = "reqwest"\nversion = "0.12.0"\n{CIO}\nchecksum = "x"\n'
        errors, _ = p.check(lock, DENY)
        self.assertEqual(len(errors), 1)
        self.assertIn("reqwest", errors[0])

    def test_git_source_is_rejected(self):
        lock = OK_LOCK + '[[package]]\nname = "x"\nversion = "1.0.0"\nsource = "git+https://example.com/x#abc"\n'
        errors, _ = p.check(lock, DENY)
        self.assertTrue(any("disallowed source" in e for e in errors), errors)

    def test_other_registry_is_rejected(self):
        lock = OK_LOCK + '[[package]]\nname = "x"\nversion = "1.0.0"\nsource = "registry+https://evil.example/index"\nchecksum = "c"\n'
        errors, _ = p.check(lock, DENY)
        self.assertTrue(any("disallowed source" in e for e in errors), errors)

    def test_missing_checksum_is_rejected(self):
        lock = OK_LOCK + f'[[package]]\nname = "y"\nversion = "1.0.0"\n{CIO}\n'
        errors, _ = p.check(lock, DENY)
        self.assertTrue(any("no checksum" in e for e in errors), errors)

    def test_missing_lock_is_not_an_error(self):
        self.assertEqual(p.main(["/nonexistent/Cargo.lock"]), 0)

    def test_deny_toml_parses_and_bans_core_network_crates(self):
        import tomllib
        banned = {e["crate"] for e in tomllib.loads(DENY)["bans"]["deny"]}
        for must in ("reqwest", "hyper", "tokio", "openssl", "rustls", "sentry"):
            self.assertIn(must, banned)


if __name__ == "__main__":
    unittest.main()
