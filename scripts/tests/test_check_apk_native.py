import struct
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import check_apk_native as c  # noqa: E402

# symbol attribute constants (ELF)
LOCAL, GLOBAL, WEAK = 0, 1, 2
NOTYPE, OBJECT, FUNC = 0, 1, 2
DEFAULT, INTERNAL, HIDDEN, PROTECTED = 0, 1, 2, 3
UNDEF = 0


def S(name, bind=GLOBAL, typ=FUNC, vis=DEFAULT, shndx=1):
    return (name, bind, typ, vis, shndx)


NAMES = ("Java_a_b_nativeInit", "Java_a_b_nativeVersion")
EXPECTED = set(NAMES)
GOOD_SYMS = [S(n) for n in NAMES] + [S("unrelated_symbol")]
MACHINE = {"arm64-v8a": 183, "x86_64": 62, "armeabi-v7a": 40}


def build_elf(bits=64, align=0x4000, dynsyms=None, symtab=(), machine=183, e_type=3, loads=None):
    """Minimal ELF shared object. Sections: 0 null, 1 .dynsym, 2 .dynstr, [3 .symtab, 4 .strtab].
    Returns (bytes, layout) so tests can corrupt individual fields."""
    dynsyms = GOOD_SYMS if dynsyms is None else dynsyms
    if bits == 64:
        ehsize, phent, shent, symsize = 64, 56, 64, 24
    else:
        ehsize, phent, shent, symsize = 52, 32, 40, 16
    loads = loads or [(0, 0, 0x100, 0x100, align), (0, 0, 0x100, 0x100, align)]

    def strtab_and_offsets(symbols):
        tab, offs = b"\0", []
        for s in symbols:
            offs.append(len(tab))
            tab += s[0].encode() + b"\0"
        return tab, offs

    def pack_syms(symbols, offs):
        out = b"\0" * symsize
        for s, off in zip(symbols, offs):
            _, bind, typ, vis, shndx = s
            info = (bind << 4) | typ
            if bits == 64:
                out += struct.pack("<IBBHQQ", off, info, vis, shndx, 0x1000, 0)
            else:
                out += struct.pack("<IIIBBH", off, 0x1000, 0, info, vis, shndx)
        return out

    dynstr, offs = strtab_and_offsets(dynsyms)
    dynsym_b = pack_syms(dynsyms, offs)
    sections_n = 3
    parts = []  # (name, bytes)
    L = SimpleNamespace(bits=bits, ehsize=ehsize, phent=phent, shent=shent, symsize=symsize, nph=len(loads))
    L.ph_off = ehsize
    cur = L.ph_off + len(loads) * phent
    L.dynsym_off = cur
    cur += len(dynsym_b)
    L.dynstr_off = cur
    cur += len(dynstr)
    symtab_b = strtab_b = b""
    if symtab:
        strtab_b, soffs = strtab_and_offsets(symtab)
        symtab_b = pack_syms(symtab, soffs)
        L.symtab_off = cur
        cur += len(symtab_b)
        L.strtab_off = cur
        cur += len(strtab_b)
        sections_n = 5
    L.sh_off = cur
    L.dynsym_len, L.dynstr_len = len(dynsym_b), len(dynstr)

    def sh(typ, off, size, link, entsize):
        if bits == 64:
            return struct.pack("<IIQQQQIIQQ", 0, typ, 2, 0, off, size, link, 0, 8, entsize)
        return struct.pack("<IIIIIIIIII", 0, typ, 2, 0, off, size, link, 0, 4, entsize)

    zero = struct.pack("<IIQQQQIIQQ", *([0] * 10)) if bits == 64 else struct.pack("<IIIIIIIIII", *([0] * 10))
    sht = zero + sh(11, L.dynsym_off, len(dynsym_b), 2, symsize) + sh(3, L.dynstr_off, len(dynstr), 0, 0)
    if symtab:
        sht += sh(2, L.symtab_off, len(symtab_b), 4, symsize) + sh(3, L.strtab_off, len(strtab_b), 0, 0)

    ident = b"\x7fELF" + bytes([2 if bits == 64 else 1, 1, 1, 0]) + b"\0" * 8
    if bits == 64:
        ehdr = ident + struct.pack("<HHIQQQIHHHHHH", e_type, machine, 1, 0, L.ph_off, L.sh_off, 0, ehsize, phent, len(loads), shent, sections_n, 0)
        phdrs = b"".join(struct.pack("<IIQQQQQQ", 1, 5, o, v, 0, fs, ms, al) for (o, v, fs, ms, al) in loads)
    else:
        ehdr = ident + struct.pack("<HHIIIIIHHHHHH", e_type, machine, 1, 0, L.ph_off, L.sh_off, 0, ehsize, phent, len(loads), shent, sections_n, 0)
        phdrs = b"".join(struct.pack("<IIIIIIII", 1, o, v, 0, fs, ms, 5, al) for (o, v, fs, ms, al) in loads)
    data = ehdr + phdrs + dynsym_b + dynstr + symtab_b + strtab_b + sht
    return bytearray(data), L


def build_apk(path: Path, libs: dict, aligned=True, compress=False):
    """Writes an APK whose lib entries are stored and (aligned=True) start on 16384-byte boundaries."""
    with zipfile.ZipFile(path, "w") as zf:
        zf.writestr("AndroidManifest.xml", b"<manifest/>")
        for name, data in libs.items():
            zi = zipfile.ZipInfo(name)
            zi.compress_type = zipfile.ZIP_DEFLATED if compress else zipfile.ZIP_STORED
            if aligned:
                start = zf.fp.tell() + 30 + len(name)
                zi.extra = b"\0" * ((-start) % 16384)
            zf.writestr(zi, bytes(data))


def check(arm64_elf, x86_elf=None, expected=EXPECTED, **apk_kw):
    """Runs the real APK check with `arm64_elf` in lib/arm64-v8a and a good (or given) x86_64 lib."""
    x86 = x86_elf if x86_elf is not None else build_elf(machine=62)[0]
    apk = Path(tempfile.mkdtemp()) / "t.apk"
    build_apk(apk, {"lib/arm64-v8a/librpx_core.so": arm64_elf, "lib/x86_64/librpx_core.so": x86}, **apk_kw)
    return c.check_apk(apk, ["arm64-v8a", "x86_64"], {"arm64-v8a", "x86_64"}, expected)


def arm(**kw):
    return build_elf(**kw)[0]


class ElfParserTests(unittest.TestCase):
    def test_elf64_alignment_and_exports(self):
        e = c.parse_elf(arm())
        self.assertEqual((e["bits"], e["machine"]), (64, 183))
        self.assertEqual(e["load_aligns"], [0x4000, 0x4000])
        self.assertEqual(e["exports"], {"Java_a_b_nativeInit", "Java_a_b_nativeVersion", "unrelated_symbol"})

    def test_elf32_is_parsed(self):
        e = c.parse_elf(build_elf(32, 0x1000, machine=40)[0])
        self.assertEqual((e["bits"], e["load_aligns"][0], e["machine"]), (32, 0x1000, 40))
        self.assertIn("Java_a_b_nativeInit", e["exports"])

    def test_garbage_is_rejected(self):
        with self.assertRaises(c.ElfError):
            c.parse_elf(b"not an elf at all")

    def test_real_system_libraries_parse(self):
        """Real linker output, not synthetic: libz exports `inflate`, segments are 4 KB aligned."""
        path = Path("/lib/x86_64-linux-gnu/libz.so.1")
        if not path.exists():
            self.skipTest("no /lib/x86_64-linux-gnu/libz.so.1 on this machine")
        e = c.parse_elf(path.read_bytes())
        self.assertIn("inflate", e["exports"])
        self.assertTrue(all(a >= 0x1000 for a in e["load_aligns"]))
        self.assertEqual(e["machine"], 62)


class GoodApkTests(unittest.TestCase):
    def test_good_apk_passes(self):
        errors, report = check(arm())
        self.assertEqual(errors, [])
        self.assertEqual(len(report), 2)

    def test_32bit_abi_is_not_held_to_16k(self):
        apk = Path(tempfile.mkdtemp()) / "t.apk"
        build_apk(apk, {"lib/armeabi-v7a/librpx_core.so": build_elf(32, 0x1000, machine=40)[0]}, aligned=False)
        errors, _ = c.check_apk(apk, ["armeabi-v7a"], {"arm64-v8a", "x86_64"}, EXPECTED)
        self.assertEqual(errors, [])


class ApkLevelNegativeTests(unittest.TestCase):
    def assertFails(self, result, *needles):
        errors, _ = result
        self.assertTrue(errors, "expected the APK check to fail")
        joined = "\n".join(errors)
        for needle in needles:
            self.assertIn(needle, joined)

    def test_4k_elf_alignment_fails(self):
        self.assertFails(check(arm(align=0x1000)), "p_align")

    def test_unaligned_zip_offset_fails(self):
        self.assertFails(check(arm(), aligned=False), "zip data offset")

    def test_compressed_library_fails(self):
        self.assertFails(check(arm(), compress=True), "compressed")

    def test_missing_abi_fails(self):
        apk = Path(tempfile.mkdtemp()) / "t.apk"
        build_apk(apk, {"lib/arm64-v8a/librpx_core.so": arm()})
        errors, _ = c.check_apk(apk, ["arm64-v8a", "x86_64"], {"arm64-v8a", "x86_64"}, EXPECTED)
        self.assertTrue(any("lib/x86_64/librpx_core.so is missing" in e for e in errors), errors)

    def test_unreadable_apk_fails(self):
        bad = Path(tempfile.mkdtemp()) / "bad.apk"
        bad.write_bytes(b"nope")
        errors, _ = c.check_apk(bad, ["arm64-v8a"], set(), EXPECTED)
        self.assertEqual(len(errors), 1)


class SymbolNegativeTests(unittest.TestCase):
    """The required symbol exists with the right name but is not a valid JNI export."""

    def broken(self, **attrs):
        syms = [S("Java_a_b_nativeInit", **attrs), S("Java_a_b_nativeVersion")]
        return check(arm(dynsyms=syms))

    def assertRejected(self, result, reason):
        errors, _ = result
        self.assertTrue(errors, "the broken symbol was accepted")
        self.assertTrue(any("nativeInit" in e and reason in e for e in errors), errors)
        self.assertFalse(any("nativeVersion" in e for e in errors), errors)

    def test_hidden_visibility(self):
        self.assertRejected(self.broken(vis=HIDDEN), "HIDDEN")

    def test_internal_visibility(self):
        self.assertRejected(self.broken(vis=INTERNAL), "INTERNAL")

    def test_protected_visibility(self):
        self.assertRejected(self.broken(vis=PROTECTED), "PROTECTED")

    def test_weak_binding(self):
        self.assertRejected(self.broken(bind=WEAK), "WEAK")

    def test_local_binding(self):
        self.assertRejected(self.broken(bind=LOCAL), "LOCAL")

    def test_undefined_symbol(self):
        self.assertRejected(self.broken(shndx=UNDEF), "SHN_UNDEF")

    def test_reserved_section_index(self):
        self.assertRejected(self.broken(shndx=0xFFF1), "reserved section index")

    def test_section_index_out_of_range(self):
        self.assertRejected(self.broken(shndx=17), "out of range")

    def test_not_a_function(self):
        self.assertRejected(self.broken(typ=OBJECT), "OBJECT")

    def test_symbol_absent_from_dynamic_table(self):
        errors, _ = check(arm(dynsyms=[S("Java_a_b_nativeVersion")]))
        self.assertTrue(any("nativeInit" in e and "missing from .dynsym" in e for e in errors), errors)

    def test_symbol_only_in_static_symtab(self):
        elf = arm(dynsyms=[S("Java_a_b_nativeVersion")], symtab=[S("Java_a_b_nativeInit")])
        errors, _ = check(elf)
        self.assertTrue(any("nativeInit" in e and "only in .symtab" in e for e in errors), errors)

    def test_name_must_match_exactly(self):
        syms = [S("Java_a_b_nativeInit_"), S("Java_a_b_nativeinit"), S("Java_a_b_nativeVersion")]
        errors, _ = check(arm(dynsyms=syms))
        self.assertTrue(any("nativeInit" in e and "missing" in e for e in errors), errors)

    def test_one_valid_alias_is_enough(self):
        syms = [S("Java_a_b_nativeInit", vis=HIDDEN), S("Java_a_b_nativeInit"), S("Java_a_b_nativeVersion")]
        self.assertEqual(check(arm(dynsyms=syms))[0], [])


class MalformedElfTests(unittest.TestCase):
    """Corrupt one structure at a time: the APK check must report an error, never raise."""

    def corrupt(self, fmt, offset, value, **build):
        data, L = build_elf(**build)
        struct.pack_into(fmt, data, offset(L) if callable(offset) else offset, value)
        return check(data)

    def assertReported(self, result, needle):
        errors, _ = result
        self.assertTrue(errors, "malformed ELF was accepted")
        joined = "\n".join(errors)
        self.assertIn("malformed ELF", joined)
        self.assertIn(needle, joined)

    # --- truncation
    def test_truncated_inside_header(self):
        self.assertReported(check(arm()[:40]), "truncated")

    def test_truncated_inside_symbol_table(self):
        data, L = build_elf()
        self.assertReported(check(data[:L.dynsym_off + 30]), "outside the file")

    def test_empty_file(self):
        self.assertReported(check(b""), "not an ELF")

    # --- header and tables
    def test_wrong_elf_type(self):
        self.assertReported(check(arm(e_type=2)), "ET_DYN")

    def test_program_header_offset_beyond_file(self):
        self.assertReported(self.corrupt("<Q", 0x20, 10**9), "program header table")

    def test_wrong_program_header_entry_size(self):
        self.assertReported(self.corrupt("<H", 0x36, 40), "program header entry size")

    def test_wrong_section_header_entry_size(self):
        self.assertReported(self.corrupt("<H", 0x3A, 40), "section header entry size")

    def test_section_header_offset_beyond_file(self):
        self.assertReported(self.corrupt("<Q", 0x28, 10**9), "section header table")

    def test_no_section_headers(self):
        self.assertReported(self.corrupt("<H", 0x3C, 0), "no section headers")

    # --- .dynsym / string table
    def test_dynsym_wrong_entry_size(self):
        self.assertReported(self.corrupt("<Q", lambda L: L.sh_off + L.shent + 0x38, 16), "entry size")

    def test_dynsym_size_not_multiple_of_entry_size(self):
        self.assertReported(self.corrupt("<Q", lambda L: L.sh_off + L.shent + 0x20, 24 * 3 + 5), "multiple")

    def test_dynsym_beyond_file(self):
        self.assertReported(self.corrupt("<Q", lambda L: L.sh_off + L.shent + 0x20, 24 * 10**6), "outside the file")

    def test_dynsym_link_out_of_range(self):
        self.assertReported(self.corrupt("<I", lambda L: L.sh_off + L.shent + 0x28, 99), "out of range")

    def test_dynsym_link_is_not_a_string_table(self):
        self.assertReported(self.corrupt("<I", lambda L: L.sh_off + L.shent + 0x28, 1), "not a string table")

    def test_string_table_beyond_file(self):
        self.assertReported(self.corrupt("<Q", lambda L: L.sh_off + 2 * L.shent + 0x20, 10**7), "string table lies outside")

    def test_symbol_name_offset_beyond_string_table(self):
        self.assertReported(self.corrupt("<I", lambda L: L.dynsym_off + L.symsize, 10**6), "outside its string table")

    def test_unterminated_symbol_name(self):
        data, L = build_elf()
        last = L.dynstr_off + L.dynstr_len - 1
        data[last] = ord("x")  # the final NUL becomes a letter
        self.assertReported(check(data), "NUL-terminated")

    def test_two_dynsym_sections(self):
        self.assertReported(self.corrupt("<I", lambda L: L.sh_off + 2 * L.shent + 4, 11), "multiple .dynsym")

    # --- PT_LOAD consistency
    def test_no_pt_load(self):
        data, L = build_elf()
        for j in range(L.nph):
            struct.pack_into("<I", data, L.ph_off + j * L.phent, 6)  # PT_PHDR
        self.assertReported(check(data), "no PT_LOAD")

    def test_align_not_power_of_two(self):
        self.assertReported(check(arm(align=0x3000)), "power of two")

    def test_vaddr_and_offset_not_congruent(self):
        self.assertReported(check(arm(loads=[(0, 0x10, 0x100, 0x100, 0x4000)])), "congruent")

    def test_filesz_larger_than_memsz(self):
        self.assertReported(check(arm(loads=[(0, 0, 0x200, 0x100, 0x4000)])), "p_filesz")

    def test_segment_beyond_file(self):
        self.assertReported(check(arm(loads=[(0, 0, 10**7, 10**7, 0x4000)])), "outside the file")

    # --- ABI / machine
    def test_machine_does_not_match_abi(self):
        errors, _ = check(arm(machine=62))  # x86_64 code in lib/arm64-v8a
        self.assertTrue(any("arm64-v8a" in e and "machine 62" in e for e in errors), errors)

    def test_32bit_elf_in_64bit_abi(self):
        errors, _ = check(build_elf(32, 0x4000, machine=183)[0])
        self.assertTrue(any("32-bit" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
