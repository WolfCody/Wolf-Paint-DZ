#!/usr/bin/env python3
"""Static check that RustBridge.kt (external funs) and jni_bridge.rs (Java_* exports) agree.

Compares: set of method names, parameter count, and Kotlin->JNI type mapping.
It does not replace RustBridgeInstrumentedTest, it catches typos before a device run.
Usage: python3 scripts/check_jni_symbols.py   (exit code 1 on mismatch)
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GRADLE = ROOT / "app/build.gradle.kts"
RUST = ROOT / "crates/core-ffi/src/jni_bridge.rs"
# The JNI package comes from the Gradle namespace, so renaming the app cannot leave a stale constant here.
PACKAGE = re.search(r'namespace\s*=\s*"([^"]+)"', GRADLE.read_text(encoding="utf-8")).group(1) + ".core"
KOTLIN = ROOT / "app/src/main/kotlin" / PACKAGE.replace(".", "/") / "RustBridge.kt"
CLASS = "RustBridge"

PARAM_TYPES = {"String": "JString", "Int": "jint", "Long": "jlong"}
RETURN_TYPES = {"String": "jstring", "Int": "jint", "Long": "jlong", "LongArray": "jlongArray"}


def mangle(text: str) -> str:
    assert "_" not in text, "underscores need _1 escapes; keep package/class names underscore-free"
    return text.replace(".", "_")


def kotlin_methods():
    out = {}
    pattern = re.compile(r"@JvmStatic\s+external\s+fun\s+(\w+)\(([^)]*)\)\s*(?::\s*(\w+))?")
    for name, params, ret in pattern.findall(KOTLIN.read_text()):
        types = [p.split(":")[1].strip() for p in params.split(",") if p.strip()]
        out[name] = (types, ret or None)
    return out


def rust_methods():
    out = {}
    prefix = f"Java_{mangle(PACKAGE)}_{CLASS}_"
    pattern = re.compile(
        r"extern\s+\"system\"\s+fn\s+" + re.escape(prefix) + r"(\w+)(?:<'local>)?\((.*?)\)\s*(?:->\s*(\w+))?\s*\{",
        re.S,
    )
    for name, params, ret in pattern.findall(RUST.read_text()):
        parts = [p.strip() for p in re.split(r",\s*(?=\w+\s*:)", params.strip()) if p.strip()]
        types = [p.split(":", 1)[1].strip().rstrip(",").replace("<'local>", "") for p in parts]
        # first two are env + class
        out[name] = (types[2:], ret or None)
    return out


def main() -> int:
    k, r = kotlin_methods(), rust_methods()
    errors = []
    for name in sorted(set(k) | set(r)):
        if name not in r:
            errors.append(f"{name}: declared in Kotlin, no Rust export")
            continue
        if name not in k:
            errors.append(f"{name}: exported by Rust, not declared in Kotlin")
            continue
        (kt, kr), (rt, rr) = k[name], r[name]
        exp_params = [PARAM_TYPES[t] for t in kt]
        exp_ret = RETURN_TYPES[kr] if kr else None
        if exp_params != rt:
            errors.append(f"{name}: parameters Kotlin->{exp_params} vs Rust {rt}")
        if exp_ret != rr:
            errors.append(f"{name}: return Kotlin->{exp_ret} vs Rust {rr}")
    if errors:
        print("JNI mismatch:")
        print("\n".join(" - " + e for e in errors))
        return 1
    print(f"JNI symbols consistent: {len(k)} methods ({', '.join(sorted(k))})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
