#!/usr/bin/env python3
"""Offline check of Cargo.lock against deny.toml: banned crate names and package sources.

Complements `cargo deny` (which also checks licenses and runs in CI): this part needs only Python, so it also
runs in the static project checks. Usage: check_lock_policy.py [Cargo.lock] [deny.toml]
Exit code 1 on a violation; exit code 0 with a note if no Cargo.lock exists.
"""
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"


def check(lock_text: str, deny_text: str):
    lock = tomllib.loads(lock_text)
    deny = tomllib.loads(deny_text)
    banned = {}
    for entry in deny.get("bans", {}).get("deny", []):
        name = entry.get("crate") or entry.get("name")
        if name:
            banned[name.split("@")[0]] = entry.get("reason", "")
    allowed_registries = {"registry+" + r for r in deny.get("sources", {}).get("allow-registry", [])} or {CRATES_IO}
    errors, count = [], 0
    for pkg in lock.get("package", []):
        count += 1
        name, source = pkg["name"], pkg.get("source")
        if name in banned:
            errors.append(f"banned crate in Cargo.lock: {name} {pkg.get('version', '')} ({banned[name]})")
        if source is None:
            continue  # workspace member (path dependency)
        if source not in allowed_registries:
            errors.append(f"{name} {pkg.get('version', '')} comes from a disallowed source: {source}")
        if "checksum" not in pkg:
            errors.append(f"{name} {pkg.get('version', '')} has no checksum in Cargo.lock")
    return errors, count


def main(argv):
    lock = Path(argv[0]) if argv else ROOT / "Cargo.lock"
    deny = Path(argv[1]) if len(argv) > 1 else ROOT / "deny.toml"
    if not lock.is_file():
        print(f"no {lock.name}: nothing to check")
        return 0
    errors, count = check(lock.read_text(encoding="utf-8"), deny.read_text(encoding="utf-8"))
    if errors:
        print("FAILED:")
        print("\n".join(" - " + e for e in errors))
        return 1
    print(f"Cargo.lock policy OK ({count} packages, no banned crates, only crates.io sources)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
