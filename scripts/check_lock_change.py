#!/usr/bin/env python3
"""Allows `cargo update --workspace` to change Cargo.lock only for the project's own crates.

A new workspace crate (for example core-raster) makes the committed Cargo.lock stale. CI then runs
`cargo update --workspace` and calls this script with the committed and the updated lock file:

    check_lock_change.py <committed Cargo.lock> <updated Cargo.lock>

Packages that come from a registry or git (they have a `source`) must be identical in both files: same
name, version, source and checksum. Only packages without a source (path crates of this workspace) may be
added, removed or changed. Any change of a third-party package is an error and must be a deliberate,
reviewed commit. Exit code 1 on a violation or on an unreadable file.
"""
import sys
import tomllib
from pathlib import Path


def load(path: Path):
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise SystemExit(f"cannot read {path}: {e}")


def third_party(lock):
    return {(p["name"], p["version"], p.get("source"), p.get("checksum")) for p in lock.get("package", []) if p.get("source")}


def workspace_crates(lock):
    return {(p["name"], p["version"]) for p in lock.get("package", []) if not p.get("source")}


def compare(old, new):
    """Returns (errors, notes)."""
    errors, notes = [], []
    old_tp, new_tp = third_party(old), third_party(new)
    for name, version, source, _ in sorted(old_tp - new_tp, key=str):
        errors.append(f"third-party package removed or changed: {name} {version} ({source})")
    for name, version, source, _ in sorted(new_tp - old_tp, key=str):
        errors.append(f"third-party package added or changed: {name} {version} ({source})")
    old_ws, new_ws = workspace_crates(old), workspace_crates(new)
    for name, version in sorted(new_ws - old_ws):
        notes.append(f"workspace crate added or changed: {name} {version}")
    for name, version in sorted(old_ws - new_ws):
        notes.append(f"workspace crate removed or changed: {name} {version}")
    if old.get("version") != new.get("version"):
        errors.append(f"lock file format version changed: {old.get('version')} -> {new.get('version')}")
    return errors, notes


def main(argv):
    if len(argv) != 2:
        print(__doc__)
        return 2
    errors, notes = compare(load(Path(argv[0])), load(Path(argv[1])))
    for note in notes:
        print("note:", note)
    if errors:
        print("FAILED: the update touched third-party packages (only workspace crates may change automatically):")
        print("\n".join(" - " + e for e in errors))
        return 1
    print("Cargo.lock change is limited to workspace crates" + ("" if notes else " (no change)"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
