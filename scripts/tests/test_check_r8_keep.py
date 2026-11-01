import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import check_r8_keep as r  # noqa: E402

B = "p.core.RustBridge"
E = "p.core.RpxNativeException"
EXP = {
    "bridge": B,
    "natives": {
        "nativeInit": ("void", ["java.lang.String"]),
        "nativeVersion": ("java.lang.String", []),
        "nativeCreateDocument": ("long", ["java.lang.String", "int", "int", "int"]),
    },
    "base": E,
    "subclasses": [E + "$InvalidArgument", E + "$Io"],
}

SEEDS = f"""{B}
{B}: void nativeInit(java.lang.String)
{B}: java.lang.String nativeVersion()
{B}: long nativeCreateDocument(java.lang.String,int,int,int)
{E}
{E}: <init>(java.lang.String)
{E}$InvalidArgument
{E}$InvalidArgument: <init>(java.lang.String)
{E}$Io
{E}$Io: <init>(java.lang.String)
"""

BRIDGE_BLOCK = f"""{B} -> {B}:
    void nativeInit(java.lang.String) -> nativeInit
    java.lang.String nativeVersion() -> nativeVersion
    long nativeCreateDocument(java.lang.String,int,int,int) -> nativeCreateDocument
    1:1:void <clinit>():12:12 -> <clinit>
"""

MAPPING = f"""# compiler: R8
# compiler_version: 9.0.0
{BRIDGE_BLOCK}{E} -> {E}:
{E}$InvalidArgument -> {E}$InvalidArgument:
{E}$Io -> {E}$Io:
p.ui.Other -> a.a:
    int count -> a
    1:3:void render(int):20:22 -> b
"""

USAGE_OK = """p.ui.Unused
p.core.DocumentInfo:
    int component3()
"""


def run(seeds=SEEDS, mapping=MAPPING, usage=USAGE_OK):
    d = Path(tempfile.mkdtemp())
    for name, text in (("seeds.txt", seeds), ("mapping.txt", mapping), ("usage.txt", usage)):
        if text is not None:
            (d / name).write_text(text)
    return r.check_dir(d, EXP)


def with_bridge_block(block: str) -> str:
    return MAPPING.replace(BRIDGE_BLOCK, block)


class R8KeepTests(unittest.TestCase):
    def assertError(self, errors, *needles):
        self.assertTrue(errors, "expected an error, check returned success")
        joined = "\n".join(errors)
        for n in needles:
            self.assertIn(n, joined)

    # ---- success and optional files
    def test_all_good(self):
        self.assertEqual(run(), [])

    def test_usage_is_optional(self):
        self.assertEqual(run(usage=None), [])

    def test_line_number_ranges_and_comments_are_understood(self):
        mapping = with_bridge_block(
            f'{B} -> {B}:\n    # {{"id":"sourceFile","fileName":"RustBridge.kt"}}\n'
            "    void nativeInit(java.lang.String) -> nativeInit\n"
            "    1:2:java.lang.String nativeVersion():5:6 -> nativeVersion\n"
            "    long nativeCreateDocument(java.lang.String,int,int,int) -> nativeCreateDocument\n"
        )
        self.assertEqual(run(mapping=mapping), [])

    # ---- the case from the review: class kept, one native method renamed
    def test_native_method_renamed_by_r8(self):
        mapping = MAPPING.replace("void nativeInit(java.lang.String) -> nativeInit", "void nativeInit(java.lang.String) -> a")
        self.assertError(run(mapping=mapping), "nativeInit", "renamed to a")

    def test_each_native_method_is_checked_individually(self):
        for name, (ret, params) in EXP["natives"].items():
            line = f"    {ret} {name}({','.join(params)}) -> {name}"
            mapping = MAPPING.replace(line, line.replace(f"-> {name}", "-> z"))
            self.assertIn(line, MAPPING)
            self.assertError(run(mapping=mapping), name, "renamed to z")

    def test_class_name_kept_but_all_methods_renamed(self):
        mapping = with_bridge_block(
            f"{B} -> {B}:\n    void nativeInit(java.lang.String) -> a\n    java.lang.String nativeVersion() -> b\n"
            "    long nativeCreateDocument(java.lang.String,int,int,int) -> c\n"
        )
        errors = run(mapping=mapping)
        self.assertEqual(len([e for e in errors if "renamed to" in e]), 3, errors)

    # ---- missing method and malformed entries
    def test_native_method_missing_from_mapping(self):
        mapping = MAPPING.replace("    java.lang.String nativeVersion() -> nativeVersion\n", "")
        self.assertError(run(mapping=mapping), "nativeVersion", "no entry")

    def test_method_entry_with_wrong_signature(self):
        mapping = MAPPING.replace("void nativeInit(java.lang.String) -> nativeInit", "void nativeInit(int) -> nativeInit")
        self.assertError(run(mapping=mapping), "nativeInit", "signature")

    def test_malformed_entry_inside_the_bridge_block(self):
        mapping = MAPPING.replace("    java.lang.String nativeVersion() -> nativeVersion\n", "    nativeVersion ==> nativeVersion\n")
        self.assertError(run(mapping=mapping), "unrecognized line", "RustBridge block")

    def test_garbage_line_outside_any_class_block(self):
        self.assertError(run(mapping="garbage that is not a mapping\n" + MAPPING), "outside any class block")

    def test_member_line_before_any_class(self):
        self.assertError(run(mapping="    void nativeInit(java.lang.String) -> nativeInit\n" + MAPPING), "unrecognized line")

    def test_unparsable_mapping_fails(self):
        self.assertError(run(mapping="# only a comment\n"), "no class blocks")

    def test_mapping_with_only_other_classes(self):
        self.assertError(run(mapping="p.ui.Other -> a.a:\n    int count -> a\n"), "missing")

    def test_unrelated_class_with_odd_line_is_tolerated(self):
        mapping = MAPPING.replace("    int count -> a\n", "    some odd line R8 may write -> x -> y\n")
        self.assertEqual(run(mapping=mapping), [])

    # ---- classes
    def test_renamed_bridge_class_fails(self):
        self.assertError(run(mapping=MAPPING.replace(f"{B} -> {B}:", f"{B} -> a.b:")), "renamed")

    def test_removed_exception_subclass_fails(self):
        self.assertError(run(mapping=MAPPING.replace(f"{E}$Io -> {E}$Io:\n", "")), "$Io", "missing")

    # ---- seeds
    def test_missing_seeds_and_mapping_fail_closed(self):
        self.assertEqual(len(run(seeds=None, mapping=None)), 2)
        self.assertTrue(run(seeds=None))
        self.assertTrue(run(mapping=None))

    def test_native_method_not_in_seeds(self):
        self.assertError(run(seeds=SEEDS.replace("nativeVersion", "somethingElse")), "seeds.txt", "nativeVersion")

    def test_seed_with_wrong_signature_is_not_accepted(self):
        self.assertError(run(seeds=SEEDS.replace("void nativeInit(java.lang.String)", "void nativeInit(int)")), "nativeInit")

    def test_exception_constructor_not_kept(self):
        self.assertError(run(seeds=SEEDS.replace(f"{E}$Io: <init>(java.lang.String)\n", "")), "$Io")

    # ---- usage
    def test_usage_lists_removed_bridge_class(self):
        self.assertError(run(usage=USAGE_OK + f"{B}\n"), "removed entirely")

    def test_usage_lists_removed_native_method(self):
        self.assertError(run(usage=f"{B}:\n    void nativeInit(java.lang.String)\n"), "nativeInit", "removed")

    def test_usage_lists_removed_exception_class(self):
        self.assertError(run(usage=USAGE_OK + f"{E}$Io\n"), "removed entirely")

    # ---- expectations come from the real sources
    def test_expectations_come_from_the_real_sources(self):
        exp = r.expectations()
        self.assertEqual(len(exp["natives"]), 8)
        self.assertEqual(exp["natives"]["nativeCreateDocument"], ("long", ["java.lang.String", "int", "int", "int"]))
        self.assertEqual(exp["natives"]["nativeDocumentInfo"], ("long[]", ["long"]))
        self.assertEqual(exp["natives"]["nativeDebugPanic"], ("void", []))
        self.assertEqual(len(exp["subclasses"]), 8)
        self.assertTrue(exp["bridge"].endswith(".core.RustBridge"))


if __name__ == "__main__":
    unittest.main()
