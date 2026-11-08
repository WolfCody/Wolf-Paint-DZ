#!/usr/bin/env python3
"""Verifies that R8 kept what JNI needs in a release build. Reads the files AGP writes to
app/build/outputs/mapping/release/ : seeds.txt, mapping.txt and (if present) usage.txt.

JNI binds a Java `native` method to the C symbol `Java_<class>_<method>` by the *names* of the class and of
the method, so keeping the class is not enough. Names derive from the sources (namespace in
app/build.gradle.kts, `external fun`s of RustBridge.kt, subclasses of RpxNativeException.kt):

  1. seeds.txt   (what the keep rules matched, i.e. what R8 must keep unrenamed):
                 RustBridge lists EVERY native method with its exact signature; every RpxNativeException subclass
                 lists its (String) constructor (Rust constructs them by name); the base class is kept.
  2. mapping.txt (what R8 renamed): RustBridge, RpxNativeException and its subclasses map to their own name.
                 R8 writes a member to mapping.txt only when it is renamed or carries line information, so a native
                 method WITHOUT an entry is fine (it is proven kept by seeds.txt). A native method WITH an entry must
                 keep its name (`nativeInit -> a` is an error) and its signature.
  3. usage.txt   (what R8 removed): none of those classes (nor RustBridge's native methods) appears as removed code.

Parsing follows the real R8 format: class names may contain `$`, `-` and any other non-blank characters (Kotlin
mangles names such as `clickable-O2vRcR0`). Only lines that can be attributed to RustBridge are checked strictly;
lines of other classes are skipped. Fails closed on: missing/empty seeds.txt or mapping.txt, a mapping without class
blocks, a corrupt top-level line, a member line before any class block, an unrecognized line inside the RustBridge
block. On failure, the relevant lines of the files are printed so that the format can be compared with the check.

Usage: check_r8_keep.py app/build/outputs/mapping/release
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

JAVA_TYPES = {"String": "java.lang.String", "Int": "int", "Long": "long", "LongArray": "long[]", "Boolean": "boolean"}

# `original.Name -> obfuscated.Name:`  (any non-blank characters: `$`, `-`, ...)
CLASS_LINE = re.compile(r"^(?P<orig>\S+) -> (?P<obf>\S+):$")
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


def context(lines, needle, limit=6):
    """Lines of a file that mention `needle`, to show the real format next to an error."""
    hits = [line.strip() for line in lines if needle in line]
    shown = hits[:limit] + ([f"... {len(hits) - limit} more"] if len(hits) > limit else [])
    return shown or ["(no line mentions it)"]


def parse_mapping(lines, tracked):
    """Single pass over mapping.txt.
    Returns (class_map, members, problems).
    class_map: original class -> obfuscated class (every class block);
    members:   original class -> list of member dicts, only for classes in `tracked`;
    problems:  (line number, text, why) for lines that cannot be attributed to anything valid."""
    class_map, members, problems = {}, {c: [] for c in tracked}, []
    current = None
    for number, line in enumerate(lines, 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line[0].isspace():
            header = CLASS_LINE.match(line)
            if header:
                current = header.group("orig")
                class_map[current] = header.group("obf")
            else:
                current = None
                problems.append((number, line, "neither a class header nor a comment"))
            continue
        if current is None:
            problems.append((number, line, "member line outside any class block"))
            continue
        if current not in tracked:
            continue  # members of unrelated classes are not our business
        m = METHOD_LINE.match(line)
        if m:
            params = [p.strip() for p in m.group("params").split(",") if p.strip()]
            members[current].append({"kind": "method", "name": m.group("name"), "obf": m.group("obf"),
                                     "ret": m.group("ret"), "params": params})
            continue
        f = FIELD_LINE.match(line)
        if f:
            members[current].append({"kind": "field", "name": f.group("name"), "obf": f.group("obf"),
                                     "ret": f.group("type"), "params": []})
            continue
        problems.append((number, line, f"unrecognized member line in the {current} block"))
    return class_map, members, problems


def constructor_pattern(cls):
    """`Class: <init>(java.lang.String)` or the ProGuard-style `Class: ShortName(java.lang.String)`."""
    names = {"<init>", cls.rsplit(".", 1)[-1], cls.rsplit("$", 1)[-1]}
    alternatives = "|".join(re.escape(n) for n in sorted(names))
    return re.compile(rf"^{re.escape(cls)}: (?:void )?(?:{alternatives})\(java\.lang\.String\)$")


def check_dir(mapping_dir: Path, exp):
    errors = []
    seeds, mapping, usage = (read(mapping_dir / n) for n in ("seeds.txt", "mapping.txt", "usage.txt"))
    if seeds is None:
        errors.append(f"{mapping_dir}/seeds.txt not found (release build with R8 required)")
    elif not any(line.strip() for line in seeds):
        errors.append("seeds.txt is empty")
    if mapping is None:
        errors.append(f"{mapping_dir}/mapping.txt not found (release build with R8 required)")
    elif not any(line.strip() for line in mapping):
        errors.append("mapping.txt is empty")
    if errors:
        return errors

    bridge, base, subs = exp["bridge"], exp["base"], exp["subclasses"]
    seed_lines = [line.rstrip().replace(", ", ",") for line in seeds]
    seed_set = set(seed_lines)

    # 1. seeds: everything JNI needs is kept
    for name, (ret, params) in exp["natives"].items():
        wanted = f"{bridge}: {ret} {name}({','.join(params)})"
        if wanted not in seed_set:
            errors.append(f"seeds.txt: native method {bridge}.{name} {signature(ret, params)} is not kept "
                          f"(expected the line {wanted!r}); lines mentioning it: {context(seed_lines, name + '(')}")
    if not any(line == base or line.startswith(base + ":") for line in seed_lines):
        errors.append(f"seeds.txt: {base} is not kept")
    for cls in subs:
        pattern = constructor_pattern(cls)
        if not any(pattern.match(line) for line in seed_lines):
            errors.append(f"seeds.txt: constructor (String) of {cls} is not kept; "
                          f"lines mentioning the class: {context(seed_lines, cls + ':')}")

    # 2. mapping: no renaming of the classes, and no renamed native method
    class_map, members, problems = parse_mapping(mapping, {bridge})
    for number, line, why in problems[:20]:
        errors.append(f"mapping.txt line {number}: {why}: {line.strip()[:100]!r}")
    if len(problems) > 20:
        errors.append(f"mapping.txt: {len(problems) - 20} more corrupt lines not shown")
    if not class_map:
        errors.append("mapping.txt: no class blocks could be parsed (unknown format?)")
        return errors
    for cls in [bridge, base, *subs]:
        if cls not in class_map:
            errors.append(f"mapping.txt: {cls} is missing (removed by R8?)")
        elif class_map[cls].startswith("R8$$REMOVED$$CLASS$$"):
            errors.append(f"mapping.txt: {cls} was removed by R8 (mapped to {class_map[cls]})")
        elif class_map[cls] != cls:
            errors.append(f"mapping.txt: {cls} was renamed to {class_map[cls]} (JNI/native lookup would break)")
    methods = [m for m in members[bridge] if m["kind"] == "method"]
    for name, (ret, params) in exp["natives"].items():
        # no entry = not renamed (R8 lists renamed members only); the seeds check above proves it was kept
        for m in (m for m in methods if m["name"] == name):
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


def summary(mapping_dir: Path, exp):
    mapping, seeds = read(mapping_dir / "mapping.txt") or [], read(mapping_dir / "seeds.txt") or []
    class_map, members, _ = parse_mapping(mapping, {exp["bridge"]})
    renamed_members = [m for m in members[exp["bridge"]] if m["obf"] != m["name"]]
    return (f"mapping.txt: {len(class_map)} classes, {len(members[exp['bridge']])} RustBridge member entries "
            f"({len(renamed_members)} renamed); seeds.txt: {len(seeds)} lines")


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
    print(summary(Path(argv[0]), exp))
    print(f"R8 keeps OK: {exp['bridge']} ({len(exp['natives'])} native methods kept with their names and signatures), "
          f"{exp['base']} and {len(exp['subclasses'])} subclasses are retained and not renamed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
