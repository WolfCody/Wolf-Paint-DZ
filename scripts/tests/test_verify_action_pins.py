import os
import stat
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import verify_action_pins as v  # noqa: E402

GOOD = "a" * 40


def make_repo(workflow: str) -> Path:
    root = Path(tempfile.mkdtemp())
    (root / ".github/workflows").mkdir(parents=True)
    (root / ".github/workflows/ci.yml").write_text(textwrap.dedent(workflow), encoding="utf-8")
    return root


class StructureTests(unittest.TestCase):
    def errors(self, wf):
        return v.check_structure(make_repo(wf))[0]

    def test_first_party_tags_and_local_actions_are_allowed(self):
        wf = """
        jobs:
          a:
            steps:
              - uses: actions/checkout@v7
              - uses: ./.github/actions/local
              - uses: docker://alpine:3
        """
        self.assertEqual(self.errors(wf), [])

    def test_third_party_tag_is_rejected(self):
        self.assertEqual(len(self.errors("jobs:\n  a:\n    steps:\n      - uses: gradle/actions/setup-gradle@v6\n")), 1)

    def test_third_party_short_sha_is_rejected(self):
        self.assertEqual(len(self.errors("jobs:\n  a:\n    steps:\n      - uses: x/y@abc1234 # v1.0.0\n")), 1)

    def test_pinned_sha_without_version_comment_is_rejected(self):
        self.assertEqual(len(self.errors(f"jobs:\n  a:\n    steps:\n      - uses: x/y@{GOOD}\n")), 1)
        self.assertEqual(len(self.errors(f"jobs:\n  a:\n    steps:\n      - uses: x/y@{GOOD} # latest\n")), 1)

    def test_valid_pin_is_accepted_and_reported(self):
        errs, pinned = v.check_structure(make_repo(f"jobs:\n  a:\n    steps:\n      - uses: gradle/actions/setup-gradle@{GOOD} # v6.1.0\n"))
        self.assertEqual(errs, [])
        self.assertEqual(pinned[0][1:], ("gradle/actions", GOOD, "v6.1.0"))

    def test_uses_without_version_is_rejected(self):
        self.assertEqual(len(self.errors("jobs:\n  a:\n    steps:\n      - uses: x/y\n")), 1)


class OnlineTests(unittest.TestCase):
    """The `git` binary is replaced by a fake that answers like `git ls-remote`."""

    def run_with_fake_git(self, body: str, pinned):
        fake_dir = tempfile.mkdtemp()
        fake = Path(fake_dir) / "git"
        fake.write_text("#!/usr/bin/env bash\n" + body + "\n")
        fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
        old = os.environ["PATH"]
        os.environ["PATH"] = fake_dir + os.pathsep + old
        try:
            return v.check_online(pinned)
        finally:
            os.environ["PATH"] = old

    def test_matching_lightweight_tag(self):
        errs = self.run_with_fake_git(f'printf "{GOOD}\\trefs/tags/v1.0.0\\n"', [("w", "x/y", GOOD, "v1.0.0")])
        self.assertEqual(errs, [])

    def test_annotated_tag_is_peeled(self):
        body = f'printf "{"b" * 40}\\trefs/tags/v1.0.0\\n{GOOD}\\trefs/tags/v1.0.0^{{}}\\n"'
        self.assertEqual(self.run_with_fake_git(body, [("w", "x/y", GOOD, "v1.0.0")]), [])

    def test_mismatching_sha_fails(self):
        errs = self.run_with_fake_git(f'printf "{"c" * 40}\\trefs/tags/v1.0.0\\n"', [("w", "x/y", GOOD, "v1.0.0")])
        self.assertEqual(len(errs), 1)
        self.assertIn("pinned", errs[0])

    def test_missing_tag_fails(self):
        errs = self.run_with_fake_git("true", [("w", "x/y", GOOD, "v9.9.9")])
        self.assertEqual(len(errs), 1)
        self.assertIn("not found", errs[0])

    def test_git_failure_fails(self):
        errs = self.run_with_fake_git("echo boom >&2; exit 128", [("w", "x/y", GOOD, "v1.0.0")])
        self.assertEqual(len(errs), 1)
        self.assertIn("cannot verify", errs[0])


if __name__ == "__main__":
    unittest.main()
