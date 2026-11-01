#!/usr/bin/env python3
"""Checks that third-party GitHub Actions are pinned to full commit SHAs.

Policy (docs/CI.md):
  * actions owned by GitHub (`actions/*`) may use release tags;
  * every other action must use a 40-hex commit SHA followed by a `# vX.Y.Z` comment naming the release;
  * local actions (`./...`) and docker:// refs are ignored.

Offline (default): structure only.
--online: also resolves the comment's tag with `git ls-remote` and requires the pinned SHA to be the
commit that tag points to (annotated tags are peeled). Needs network and `git`.
Exit code 1 on any violation.
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
USES = re.compile(r"^\s*(?:-\s+)?uses:\s*([^\s#]+)\s*(?:#\s*(\S+))?\s*$")
FIRST_PARTY_OWNERS = {"actions"}
SHA = re.compile(r"^[0-9a-f]{40}$")
TAG = re.compile(r"^v\d+\.\d+\.\d+$")


def workflow_files(root: Path):
    yield from sorted((root / ".github/workflows").glob("*.y*ml"))
    yield from sorted((root / ".github/actions").glob("**/action.y*ml"))


def collect(root: Path):
    """Returns [(file, line_no, ref, comment)] for every `uses:`."""
    found = []
    for path in workflow_files(root):
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            m = USES.match(line)
            if m:
                found.append((path.relative_to(root).as_posix(), n, m.group(1), m.group(2)))
    return found


def split_ref(ref: str):
    action, _, version = ref.partition("@")
    parts = action.split("/")
    return action, version, parts[0], "/".join(parts[:2])


def check_structure(root: Path):
    errors, pinned = [], []
    for file, n, ref, comment in collect(root):
        if ref.startswith(("./", "docker://")):
            continue
        action, version, owner, repo = split_ref(ref)
        where = f"{file}:{n} {ref}"
        if "@" not in ref:
            errors.append(f"{where}: no version")
        elif owner in FIRST_PARTY_OWNERS:
            continue
        elif not SHA.match(version):
            errors.append(f"{where}: third-party action must be pinned to a 40-hex commit SHA")
        elif not comment or not TAG.match(comment):
            errors.append(f"{where}: pinned SHA needs a trailing '# vX.Y.Z' comment naming the release")
        else:
            pinned.append((where, repo, version, comment))
    return errors, pinned


def resolve_tag(repo: str, tag: str):
    """Commit SHA the tag points to (peeled), or None."""
    out = subprocess.run(
        ["git", "ls-remote", f"https://github.com/{repo}.git", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}"],
        capture_output=True, text=True, timeout=60,
    )
    if out.returncode != 0:
        raise RuntimeError(f"git ls-remote failed for {repo}: {out.stderr.strip()}")
    refs = {}
    for line in out.stdout.splitlines():
        sha, _, ref = line.partition("\t")
        refs[ref.strip()] = sha.strip()
    return refs.get(f"refs/tags/{tag}^{{}}") or refs.get(f"refs/tags/{tag}")


def check_online(pinned):
    errors = []
    for where, repo, sha, tag in pinned:
        try:
            actual = resolve_tag(repo, tag)
        except (RuntimeError, OSError, subprocess.TimeoutExpired) as e:
            errors.append(f"{where}: cannot verify online: {e}")
            continue
        if actual is None:
            errors.append(f"{where}: tag {tag} not found in {repo}")
        elif actual != sha:
            errors.append(f"{where}: pinned {sha} but {repo} {tag} is {actual}")
        else:
            print(f"ok  {repo}@{tag} == {sha}")
    return errors


def main(argv):
    errors, pinned = check_structure(ROOT)
    if "--online" in argv and not errors:
        errors += check_online(pinned)
    if errors:
        print("FAILED:")
        print("\n".join(" - " + e for e in errors))
        return 1
    print(f"action pins: OK ({len(pinned)} third-party action(s) pinned" + (", verified online)" if "--online" in argv else ")"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
