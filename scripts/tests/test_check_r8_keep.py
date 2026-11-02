import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import check_r8_keep as r  # noqa: E402

EXP = r.expectations()  # real names, derived from the project sources
B, E = EXP["bridge"], EXP["base"]

# ---------------------------------------------------------------------------------------------------------------
# REAL fragments, copied verbatim from the maintainer's report of the R8 9.3.16 output of CI run #6
# (the full artifacts were not readable by the author; everything else below is synthetic and says so).
# ---------------------------------------------------------------------------------------------------------------
REAL_MAPPING_BRIDGE_LINE = "com.dzekiwolf.wolfpaintdz.core.RustBridge -> com.dzekiwolf.wolfpaintdz.core.RustBridge:"
REAL_MAPPING_COMPOSE_LINE = (
    "androidx.compose.foundation.ClickableKt$clickable-O2vRcR0$$inlined$clickableWithIndicationIfNeeded$1 -> dh:"
)
REAL_SEEDS_FRAGMENT = """com.dzekiwolf.wolfpaintdz.core.RustBridge
com.dzekiwolf.wolfpaintdz.core.RustBridge: java.lang.String nativeVersion()
com.dzekiwolf.wolfpaintdz.core.RustBridge: void nativeInit(java.lang.String)
com.dzekiwolf.wolfpaintdz.core.RustBridge: long nativeCreateDocument(java.lang.String,int,int,int)
com.dzekiwolf.wolfpaintdz.core.RustBridge: long[] nativeOpenDocuments()
"""


def sig(name):
    ret, params = EXP["natives"][name]
    return f"{ret} {name}({','.join(params)})"


def make_seeds(drop=(), ctor_style="<init>", extra=""):
    """Synthetic seeds.txt for the real names. `drop`: native method names to leave out."""
    lines = [B] + [f"{B}: {sig(n)}" for n in EXP["natives"] if n not in drop]
    lines += [E, f"{E}: {_ctor(E, ctor_style)}(java.lang.String)"]
    for cls in EXP["subclasses"]:
        lines += [cls, f"{cls}: {_ctor(cls, ctor_style)}(java.lang.String)"]
    return "\n".join(lines) + "\n" + extra


def _ctor(cls, style):
    return {"<init>": "<init>", "short": cls.rsplit(".", 1)[-1], "nested": cls.rsplit("$", 1)[-1], "void": "void <init>"}[style]


def make_mapping(bridge_members=None, bridge_line=REAL_MAPPING_BRIDGE_LINE, extra="", drop_class=None):
    """Synthetic mapping.txt: v2.2-style header, the REAL bridge header line, kept members as R8 writes them
    (only line-numbered/renamed ones), exception classes, Compose classes with `$` and `-`."""
    if bridge_members is None:
        bridge_members = ["    1:1:void <clinit>():12:12 -> <clinit>",
                          f"    {B} INSTANCE -> INSTANCE"]
    lines = ["# compiler: R8", "# compiler_version: 9.3.16", "# min_api: 26",
             '# {"id":"com.android.tools.r8.mapping","version":"2.2"}',
             bridge_line, *bridge_members,
             f"{E} -> {E}:"]
    lines += [f"{c} -> {c}:" for c in EXP["subclasses"] if c != drop_class]
    lines += [REAL_MAPPING_COMPOSE_LINE,
              "    int label -> a",
              "    1:3:void invoke-d-I3Yd8(androidx.compose.ui.Modifier,int):10:12 -> b",
              "androidx.compose.foundation.layout.RowKt$Row-VDPg8D4$$inlined$Foo$lambda$1 -> ei:",
              "    # {\"id\":\"sourceFile\",\"fileName\":\"Row.kt\"}",
              "com.dzekiwolf.wolfpaintdz.ui.gallery.GalleryViewModel -> m3.a:"]
    return "\n".join(lines) + "\n" + extra


USAGE_OK = """androidx.compose.material3.Unused
com.dzekiwolf.wolfpaintdz.core.DocumentInfo:
    int component3()
"""


def run(seeds=None, mapping=None, usage=USAGE_OK, seeds_default=True, mapping_default=True):
    d = Path(tempfile.mkdtemp())
    seeds = make_seeds() if (seeds is None and seeds_default) else seeds
    mapping = make_mapping() if (mapping is None and mapping_default) else mapping
    for name, text in (("seeds.txt", seeds), ("mapping.txt", mapping), ("usage.txt", usage)):
        if text is not None:
            (d / name).write_text(text)
    return r.check_dir(d, EXP)


def with_bridge_members(members):
    return make_mapping(bridge_members=members)


class RealFormatTests(unittest.TestCase):
    """Behaviour on the real R8 9.3.16 output as far as it is known."""

    def test_the_five_real_seeds_lines_are_exactly_what_the_check_expects(self):
        expected = set(make_seeds().splitlines())
        for line in REAL_SEEDS_FRAGMENT.splitlines():
            self.assertIn(line, expected)

    def test_real_bridge_header_line_is_parsed(self):
        class_map, members, problems = r.parse_mapping([REAL_MAPPING_BRIDGE_LINE], {B})
        self.assertEqual(class_map, {B: B})
        self.assertEqual((members[B], problems), ([], []))

    def test_real_compose_class_with_hyphen_and_dollars_is_a_valid_class_line(self):
        class_map, _, problems = r.parse_mapping([REAL_MAPPING_COMPOSE_LINE], {B})
        self.assertEqual(problems, [])
        self.assertEqual(class_map["androidx.compose.foundation.ClickableKt$clickable-O2vRcR0$$inlined$clickableWithIndicationIfNeeded$1"], "dh")

    def test_unrenamed_native_methods_missing_from_mapping_are_fine_when_seeds_keep_them(self):
        # the situation of the real run: RustBridge block present, most native methods not listed in mapping.txt
        self.assertEqual(run(), [])

    def test_real_seeds_fragment_alone_is_not_enough(self):
        errors = run(seeds=REAL_SEEDS_FRAGMENT)
        joined = "\n".join(errors)
        self.assertIn("nativeDocumentInfo", joined)  # not among the five quoted lines
        self.assertNotIn("native method com.dzekiwolf.wolfpaintdz.core.RustBridge.nativeInit ", joined)

    def test_mapping_with_header_comments_and_json_comment_lines(self):
        self.assertEqual(run(), [])

    def test_crlf_line_endings(self):
        self.assertEqual(run(seeds=make_seeds().replace("\n", "\r\n"), mapping=make_mapping().replace("\n", "\r\n")), [])


class ClassNameTests(unittest.TestCase):
    def test_dollar_hyphen_and_mixed_names_are_class_lines(self):
        for name in ("a.B$C", "a.b.C$d-e$$inlined$f$1", "a.b.Ünïcode$x-y", "x.y-z.W"):
            class_map, _, problems = r.parse_mapping([f"{name} -> q.r:"], set())
            self.assertEqual((class_map, problems), ({name: "q.r"}, []), name)

    def test_nested_exception_classes_are_checked_with_their_dollar_names(self):
        self.assertTrue(all("$" in c for c in EXP["subclasses"]))
        self.assertEqual(run(), [])
        errors = run(mapping=make_mapping(drop_class=EXP["subclasses"][0]))
        self.assertTrue(any(EXP["subclasses"][0] in e and "missing" in e for e in errors), errors)

    def test_nested_exception_class_renamed(self):
        cls = EXP["subclasses"][1]
        mapping = make_mapping().replace(f"{cls} -> {cls}:", f"{cls} -> a.b:")
        errors = run(mapping=mapping)
        self.assertTrue(any(cls in e and "renamed to a.b" in e for e in errors), errors)

    def test_large_mapping_is_fast(self):
        extra = "".join(f"androidx.compose.Foo{i}$bar-{i:x}$$inlined$x$1 -> a{i}:\n    int f -> a\n" for i in range(60000))
        start = time.time()
        self.assertEqual(run(mapping=make_mapping(extra=extra)), [])
        self.assertLess(time.time() - start, 10)


class RenamingTests(unittest.TestCase):
    def assertError(self, errors, *needles):
        self.assertTrue(errors, "expected an error, check returned success")
        joined = "\n".join(errors)
        for n in needles:
            self.assertIn(n, joined)

    def test_native_method_renamed_by_r8(self):
        errors = run(mapping=with_bridge_members([f"    {sig('nativeInit')} -> a"]))
        self.assertError(errors, "nativeInit", "renamed to a")

    def test_each_native_method_is_checked_individually(self):
        for name in EXP["natives"]:
            errors = run(mapping=with_bridge_members([f"    {sig(name)} -> z"]))
            self.assertError(errors, name, "renamed to z")

    def test_unchanged_explicit_entries_are_accepted(self):
        members = [f"    {sig(n)} -> {n}" for n in EXP["natives"]]
        self.assertEqual(run(mapping=with_bridge_members(members)), [])

    def test_entry_with_line_numbers_and_unchanged_name_is_accepted(self):
        self.assertEqual(run(mapping=with_bridge_members([f"    1:2:{sig('nativeVersion')}:5:6 -> nativeVersion"])), [])

    def test_explicit_entry_with_wrong_signature(self):
        errors = run(mapping=with_bridge_members(["    void nativeInit(int) -> nativeInit"]))
        self.assertError(errors, "nativeInit", "signature")

    def test_all_methods_renamed_while_the_class_is_kept(self):
        members = [f"    {sig(n)} -> {chr(97 + i)}" for i, n in enumerate(EXP["natives"])]
        errors = run(mapping=with_bridge_members(members))
        self.assertEqual(len([e for e in errors if "renamed to" in e]), len(EXP["natives"]), errors)

    def test_renamed_bridge_class_fails(self):
        errors = run(mapping=make_mapping(bridge_line=f"{B} -> a.b:"))
        self.assertError(errors, "renamed to a.b")

    def test_bridge_class_missing_from_mapping(self):
        mapping = make_mapping().replace(REAL_MAPPING_BRIDGE_LINE + "\n", "")
        self.assertError(run(mapping=mapping), "RustBridge is missing")

    def test_removed_exception_subclass_fails(self):
        self.assertError(run(mapping=make_mapping(drop_class=EXP["subclasses"][-1])), "missing")

    def test_renamed_base_exception_class_fails(self):
        mapping = make_mapping().replace(f"{E} -> {E}:", f"{E} -> a:")
        self.assertError(run(mapping=mapping), "renamed to a")


class SeedsTests(unittest.TestCase):
    def assertError(self, errors, *needles):
        self.assertTrue(errors, "expected an error, check returned success")
        joined = "\n".join(errors)
        for n in needles:
            self.assertIn(n, joined)

    def test_required_native_method_missing_from_seeds(self):
        for name in EXP["natives"]:
            self.assertError(run(seeds=make_seeds(drop=(name,))), "seeds.txt", name, "is not kept")

    def test_error_shows_the_expected_line_for_comparison(self):
        errors = run(seeds=make_seeds(drop=("nativeVersion",)))
        self.assertError(errors, "expected the line", f"{B}: java.lang.String nativeVersion()")

    def test_seed_with_wrong_signature_is_not_accepted(self):
        seeds = make_seeds().replace("void nativeInit(java.lang.String)", "void nativeInit(int)")
        self.assertError(run(seeds=seeds), "nativeInit")

    def test_truncated_seed_line(self):
        seeds = make_seeds().replace("void nativeInit(java.lang.String)", "void nativeInit(java.lang.String")
        self.assertError(run(seeds=seeds), "nativeInit")

    def test_seed_without_signature(self):
        seeds = make_seeds().replace(f"{B}: void nativeInit(java.lang.String)", f"{B}: nativeInit")
        self.assertError(run(seeds=seeds), "nativeInit")

    def test_seed_of_a_different_class_is_not_accepted(self):
        seeds = make_seeds().replace(f"{B}: void nativeInit", "other.Bridge: void nativeInit")
        self.assertError(run(seeds=seeds), "nativeInit")

    def test_garbage_seeds_file(self):
        self.assertError(run(seeds="\x00\x01 not a seeds file\n???\n"), "is not kept")

    def test_empty_seeds_file(self):
        self.assertError(run(seeds=""), "seeds.txt is empty")
        self.assertError(run(seeds="\n\n"), "seeds.txt is empty")

    def test_exception_constructor_not_kept(self):
        cls = EXP["subclasses"][2]
        seeds = make_seeds().replace(f"{cls}: <init>(java.lang.String)\n", "")
        self.assertError(run(seeds=seeds), cls, "constructor")

    def test_base_exception_not_kept(self):
        seeds = "\n".join(l for l in make_seeds().splitlines() if not l.startswith(E + ":") and l != E)
        self.assertError(run(seeds=seeds + "\n"), f"{E} is not kept")

    def test_constructor_may_be_printed_in_any_known_style(self):
        for style in ("<init>", "short", "nested", "void"):
            self.assertEqual(run(seeds=make_seeds(ctor_style=style)), [], style)

    def test_constructor_with_other_parameters_is_not_accepted(self):
        cls = EXP["subclasses"][0]
        seeds = make_seeds().replace(f"{cls}: <init>(java.lang.String)", f"{cls}: <init>(int)")
        self.assertError(run(seeds=seeds), cls)


class CorruptMappingTests(unittest.TestCase):
    def assertError(self, errors, *needles):
        self.assertTrue(errors, "expected an error, check returned success")
        joined = "\n".join(errors)
        for n in needles:
            self.assertIn(n, joined)

    def test_truncated_class_header_without_colon(self):
        mapping = make_mapping(bridge_line=REAL_MAPPING_BRIDGE_LINE.rstrip(":"))
        self.assertError(run(mapping=mapping), "neither a class header nor a comment", "RustBridge is missing")

    def test_class_line_without_arrow(self):
        mapping = make_mapping(bridge_line=f"{B} => {B}:")
        self.assertError(run(mapping=mapping), "neither a class header nor a comment")

    def test_garbage_line_between_class_blocks(self):
        mapping = make_mapping(extra="this is not a mapping line\n")
        self.assertError(run(mapping=mapping), "neither a class header nor a comment")

    def test_member_line_before_any_class(self):
        mapping = "    void nativeInit(java.lang.String) -> nativeInit\n" + make_mapping()
        self.assertError(run(mapping=mapping), "member line outside any class block")

    def test_unrecognized_line_inside_the_bridge_block(self):
        errors = run(mapping=with_bridge_members(["    nativeVersion ==> nativeVersion"]))
        self.assertError(errors, "unrecognized member line", "RustBridge block")

    def test_binary_garbage(self):
        self.assertError(run(mapping="\x00\x01\x02\xff garbage\n"), "neither a class header nor a comment")

    def test_only_comments(self):
        self.assertError(run(mapping="# only a comment\n"), "no class blocks")

    def test_empty_mapping(self):
        self.assertError(run(mapping=""), "mapping.txt is empty")

    def test_mapping_with_only_other_classes(self):
        self.assertError(run(mapping="p.ui.Other -> a.a:\n    int count -> a\n"), "missing")

    def test_odd_lines_of_unrelated_classes_are_tolerated(self):
        mapping = make_mapping().replace("    int label -> a\n", "    some odd line R8 may write -> x -> y\n")
        self.assertEqual(run(mapping=mapping), [])


class FilesAndUsageTests(unittest.TestCase):
    def assertError(self, errors, *needles):
        self.assertTrue(errors, "expected an error, check returned success")
        joined = "\n".join(errors)
        for n in needles:
            self.assertIn(n, joined)

    def test_missing_seeds_and_mapping_fail_closed(self):
        d = Path(tempfile.mkdtemp())
        self.assertEqual(len(r.check_dir(d, EXP)), 2)
        self.assertTrue(run(seeds=None, seeds_default=False))
        self.assertTrue(run(mapping=None, mapping_default=False))

    def test_usage_is_optional(self):
        self.assertEqual(run(usage=None), [])

    def test_usage_lists_removed_bridge_class(self):
        self.assertError(run(usage=USAGE_OK + f"{B}\n"), "removed entirely")

    def test_usage_lists_removed_native_method(self):
        self.assertError(run(usage=f"{B}:\n    void nativeInit(java.lang.String)\n"), "nativeInit", "removed")

    def test_usage_lists_removed_exception_class(self):
        self.assertError(run(usage=USAGE_OK + f"{EXP['subclasses'][0]}\n"), "removed entirely")

    def test_usage_with_hyphenated_classes_is_fine(self):
        self.assertEqual(run(usage=USAGE_OK + "androidx.compose.Foo$bar-O2vRcR0$$inlined$x$1\n"), [])

    def test_expectations_come_from_the_real_sources(self):
        self.assertEqual(len(EXP["natives"]), 8)
        self.assertEqual(EXP["natives"]["nativeCreateDocument"], ("long", ["java.lang.String", "int", "int", "int"]))
        self.assertEqual(EXP["natives"]["nativeOpenDocuments"], ("long[]", []))
        self.assertEqual(len(EXP["subclasses"]), 8)
        self.assertTrue(EXP["bridge"].endswith(".core.RustBridge"))


if __name__ == "__main__":
    unittest.main()
