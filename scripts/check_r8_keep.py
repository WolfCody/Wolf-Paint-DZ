#!/usr/bin/env python3
"""Verifies that R8 kept what JNI needs in a release build. Reads the files AGP writes to
app/build/outputs/mapping/release/ : seeds.txt, mapping.txt and (if present) usage.txt.

JNI binds a Java `native` method to the C symbol `Java_<class>_<method>` by the *names* of the class and of
the method, so keeping the class is not enough. Names derive from the sources (namespace in
app/build.gradle.kts, `external fun`s of RustBridge.kt, subclasses of RpxNativeException.kt) and are checked:

  1. seeds.txt   : RustBridge lists every native method with its exact signature; every RpxNativeException
                   subclass lists <init>(java.lang.String) (Rust constructs them by name); the base class is kept.
  2. mapping.txt : the class blocks are parsed. RustBridge, RpxNativeException and its subclasses map to their own
                   name. Inside the RustBridge block EVERY native method has an entry whose original name equals
                   the final name (a method renamed by R8, e.g. nativeInit -> a, is an error) and whose signature
                   is the expected one. A native method without an entry is an error (removed, or not listed by R8).
                   Lines inside the RustBridge block that are neither a member entry nor a comment are an error.
  3. usage.txt   : none of those classes (nor RustBridge's native methods) appears as removed code.
Fails closed: a missing seeds.txt/mapping.txt, a mapping without class blocks, or an unrecognized line in the
RustBridge block is an error, never a success.

The file formats follow the ProGuard manual (R8 writes the same ones); the logic is tested on samples written from
that description. Whether R8 lists every unrenamed member in mapping.txt must be confirmed on the first real CI
release build; if it omits them, this check will report "no entry" and the policy has to be revisited with a real
mapping.txt in hand.

Usage: check_r8_keep.py app/build/outputs/mapping/release
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

JAVA_TYPES = {"String": "java.lang.String", "Int": "int", "Long": "long", "LongArray": "long[]"}

CLASS_LINE = re.compile(r"^([\w.$]+) -> ([\w.$]+):$")
# [startLine:endLine:]returnType name(argTypes)[:origStart[:origEnd]] -> obfuscatedName
METHOD_LINE = re.compile(
    r"^\s+(?:\d+:\d+:)?(?P<ret>\S+) (?P<name>[^\s(]+)\((?P<params>[^)]*)\)(?::\d+(?::\d+)?)? -> (?P<obf>\S+)$"
)
FIELD_LINE = re.compile(r"^\s+(?P<type>\S+) (?P<name>[^\s(]+) -> (?P<obf>\S+)$")


def expectations():
    import check_jni_symbols as jni
    package = jni.PACKAGE  # <namespace>.core
    natives = {}
    for name, (params, ret) in jni.kotlin_methods().items():
        natives[name] = (JAVA_TYPES[ret] if ret else "void", [JAVA_TYPES[p] for p in params])
    base = f"{package}.RpxNativeException"
    kt = (ROOT / "app/src/main/kotlin" / package.replace(".", "/") / "RpxNativeException.kt").read_text(encoding="utf-8")
    subs = re.findall(r"class (\w+)\(message: String\) : RpxNativeException", kt)
    return {
        "bridge": f"{package}.{jni.CLASS}",
        "natives": natives,  # name -> (return type, [parameter types]) in Java notation
        "base": base,
        "subclasses": [f"{base}${s}" for s in subs],
    }


def read(path: Path):
    return path.read_text(encoding="utf-8", errors="replace").splitlines() if path.is_file() else None


def signature(ret, params):
    return f"{ret} ({', '.join(params)})"


def parse_mapping(lines):
    """Returns (class_map, blocks, problems).
    class_map: original class -> obfuscated class; blocks: original class -> list of member dicts
    (kind 'method'/'field', original name, obfuscated name, ret/type, params) or unparsable markers."""
    class_map, blocks, problems = {}, {}, []
    current = None
    for number, line in enumerate(lines, 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        header = CLASS_LINE.match(line)
        if header:
            current = header.group(1)
            class_map[current] = header.group(2)
            blocks[current] = []
            continue
        if not line[0].isspace():
            current = None
            problems.append((None, number, line))
            continue
        if current is None:
            problems.append((None, number, line))
            continue
        m = METHOD_LINE.match(line)
        if m:
            params = [p.strip() for p in m.group("params").split(",") if p.strip()]
            blocks[current].append({"kind": "method", "name": m.group("name"), "obf": m.group("obf"),
                                    "ret": m.group("ret"), "params": params})
            continue
        f = FIELD_LINE.match(line)
        if f:
            blocks[current].append({"kind": "field", "name": f.group("name"), "obf": f.group("obf"), "ret": f.group("type"), "params": []})
            continue
        problems.append((current, number, line))
    return class_map, blocks, problems


def check_dir(mapping_dir: Path, exp):
    errors = []
    seeds, mapping, usage = (read(mapping_dir / n) for n in ("seeds.txt", "mapping.txt", "usage.txt"))
    if seeds is None:
        errors.append(f"{mapping_dir}/seeds.txt not found (release build with R8 required)")
    if mapping is None:
        errors.append(f"{mapping_dir}/mapping.txt not found (release build with R8 required)")
    if errors:
        return errors

    bridge, base, subs = exp["bridge"], exp["base"], exp["subclasses"]

    # 1. seeds
    for name, (ret, params) in exp["natives"].items():
        pat = re.compile(rf"^{re.escape(bridge)}: {re.escape(ret)} {re.escape(name)}\({re.escape(','.join(params))}\)$")
        if not any(pat.match(line.replace(", ", ",")) for line in seeds):
            errors.append(f"seeds.txt: native method {bridge}.{name} {signature(ret, params)} is not kept")
    for cls in subs:
        if not any(re.match(rf"^{re.escape(cls)}: <init>\(java\.lang\.String\)$", line) for line in seeds):
            errors.append(f"seeds.txt: constructor (String) of {cls} is not kept")
    if not any(line.strip() == base or line.startswith(base + ":") for line in seeds):
        errors.append(f"seeds.txt: {base} is not kept")

    # 2. mapping
    class_map, blocks, problems = parse_mapping(mapping)
    if not class_map:
        errors.append("mapping.txt: no class blocks could be parsed (unknown format?)")
        return errors
    for owner, number, line in problems:
        if owner is None or owner == bridge:  # lines of unrelated classes are not our business
            errors.append(f"mapping.txt line {number}: unrecognized line {line.strip()[:80]!r}"
                          + (f" in the {bridge} block" if owner else " outside any class block"))
    for cls in [bridge, base, *subs]:
        if cls not in class_map:
            errors.append(f"mapping.txt: {cls} is missing (removed by R8?)")
        elif class_map[cls] != cls:
            errors.append(f"mapping.txt: {cls} was renamed to {class_map[cls]} (JNI/native lookup would break)")

    if bridge in blocks:
        methods = [m for m in blocks[bridge] if m["kind"] == "method"]
        for name, (ret, params) in exp["natives"].items():
            entries = [m for m in methods if m["name"] == name]
            if not entries:
                errors.append(f"mapping.txt: {bridge}.{name} has no entry (removed by R8, or not listed by R8)")
                continue
            for m in entries:
                if m["obf"] != name:
                    errors.append(f"mapping.txt: native method {bridge}.{name} was renamed to {m['obf']} "
                                  f"(JNI symbol Java_..._{name} would no longer bind)")
                if (m["ret"], m["params"]) != (ret, params):
                    errors.append(f"mapping.txt: {bridge}.{name} has signature {signature(m['ret'], m['params'])}, "
                                  f"expected {signature(ret, params)}")

    # 3. usage (removed code), optional file
    if usage is not None:
        current = None
        for line in usage:
            if line and not line[0].isspace():
                current = line.rstrip(":").strip()
                if current in [bridge, base, *subs] and not line.endswith(":"):
                    errors.append(f"usage.txt: {current} was removed entirely")
            elif current == bridge:
                for name in exp["natives"]:
                    if re.search(rf"\b{re.escape(name)}\(", line):
                        errors.append(f"usage.txt: {bridge}.{name} was removed")
    return errors


def main(argv):
    if len(argv) != 1:
        print(__doc__)
        return 2
    exp = expectations()
    errors = check_dir(Path(argv[0]), exp)
    if errors:
        print("FAILED:")
        print("\n".join(" - " + e for e in errors))
        return 1
    print(f"R8 keeps OK: {exp['bridge']} ({len(exp['natives'])} native methods with their names and signatures), "
          f"{exp['base']} and {len(exp['subclasses'])} subclasses are retained and not renamed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
