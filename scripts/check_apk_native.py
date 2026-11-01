#!/usr/bin/env python3
"""Checks the native libraries inside a built APK. No Android SDK needed (pure Python).

For every lib/<abi>/librpx_core.so (abis from --abis, default `rpx.rustAbis` in gradle.properties):
  1. the library is present and stored (not compressed) in the zip;
  2. it is a well-formed ELF shared object (ET_DYN) of the machine that belongs to the ABI; all table
     bounds, entry sizes, section indices, string-table bounds and PT_LOAD segments are validated, and a
     malformed file is reported as an error, never as a traceback;
  3. for 64-bit ABIs (--require-16k, default arm64-v8a x86_64): the zip data offset is a multiple of 16384
     and every PT_LOAD has p_align >= 16384 (Android 15+ devices with 16 KB pages);
  4. every `Java_<package>_core_RustBridge_<method>` implied by the Kotlin `external fun`s is a valid JNI
     export: it is in the dynamic symbol table (.dynsym, SHT_DYNSYM) and has binding GLOBAL, visibility
     DEFAULT, type FUNC and a defined section index (not SHN_UNDEF, not reserved, in range). WEAK, LOCAL,
     HIDDEN, INTERNAL, PROTECTED, undefined and non-function symbols are rejected, as are symbols that exist
     only in .symtab.
Exit code 1 on any violation.

Usage: check_apk_native.py app/build/outputs/apk/release/app-release-unsigned.apk
"""
import argparse
import struct
import sys
import zipfile
import zlib
from collections import namedtuple
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

PAGE_16K = 16384
ET_DYN = 3
PT_LOAD = 1
SHT_SYMTAB, SHT_STRTAB, SHT_DYNSYM = 2, 3, 11
SHN_UNDEF, SHN_LORESERVE = 0, 0xFF00
STB_GLOBAL = 1
STT_FUNC = 2
STV_DEFAULT = 0

BIND_NAMES = {0: "LOCAL", 1: "GLOBAL", 2: "WEAK", 10: "GNU_UNIQUE"}
TYPE_NAMES = {0: "NOTYPE", 1: "OBJECT", 2: "FUNC", 3: "SECTION", 4: "FILE", 6: "TLS", 10: "GNU_IFUNC"}
VIS_NAMES = {0: "DEFAULT", 1: "INTERNAL", 2: "HIDDEN", 3: "PROTECTED"}
# ABI -> (e_machine, ELF bits)
ABI_MACHINE = {"arm64-v8a": (183, 64), "x86_64": (62, 64), "armeabi-v7a": (40, 32), "x86": (3, 32)}

Sym = namedtuple("Sym", "name bind type vis shndx")


class ElfError(Exception):
    pass


def _unpack(fmt, data, off, what):
    try:
        return struct.unpack_from(fmt, data, off)
    except struct.error:
        raise ElfError(f"truncated ELF: cannot read {what} at offset {off}") from None


def symbol_problems(sym: Sym, shnum: int):
    """Reasons why `sym` is not a valid JNI export (empty list = valid)."""
    problems = []
    if sym.shndx == SHN_UNDEF:
        problems.append("undefined (SHN_UNDEF)")
    elif sym.shndx >= SHN_LORESERVE:
        problems.append(f"reserved section index {sym.shndx:#x}")
    elif sym.shndx >= shnum:
        problems.append(f"section index {sym.shndx} is out of range ({shnum} sections)")
    if sym.bind != STB_GLOBAL:
        problems.append(f"binding {BIND_NAMES.get(sym.bind, sym.bind)} (need GLOBAL)")
    if sym.vis != STV_DEFAULT:
        problems.append(f"visibility {VIS_NAMES.get(sym.vis, sym.vis)} (need DEFAULT)")
    if sym.type != STT_FUNC:
        problems.append(f"type {TYPE_NAMES.get(sym.type, sym.type)} (need FUNC)")
    return problems


def _read_symbols(data, section, sections, bits, what):
    sh_type, sh_off, sh_size, sh_link, sh_entsize = section
    sym_size = 24 if bits == 64 else 16
    n = len(data)
    if sh_entsize != sym_size:
        raise ElfError(f"{what}: entry size {sh_entsize}, expected {sym_size}")
    if sh_off > n or sh_size > n - sh_off:
        raise ElfError(f"{what}: table (offset {sh_off}, size {sh_size}) lies outside the file")
    if sh_size % sym_size:
        raise ElfError(f"{what}: size {sh_size} is not a multiple of the entry size {sym_size}")
    if sh_link >= len(sections):
        raise ElfError(f"{what}: string table index {sh_link} is out of range")
    st_type, st_off, st_size, _l, _e = sections[sh_link]
    if st_type != SHT_STRTAB:
        raise ElfError(f"{what}: linked section {sh_link} is not a string table")
    if st_off > n or st_size > n - st_off:
        raise ElfError(f"{what}: string table lies outside the file")
    strtab = data[st_off:st_off + st_size]
    symbols = []
    for i in range(sh_size // sym_size):
        off = sh_off + i * sym_size
        if bits == 64:
            st_name, st_info, st_other, st_shndx = _unpack("<IBBH", data, off, f"{what} entry {i}")
        else:
            st_name, _value, _size, st_info, st_other, st_shndx = _unpack("<IIIBBH", data, off, f"{what} entry {i}")
        if st_name >= len(strtab) and i > 0:
            raise ElfError(f"{what}: symbol {i} name offset {st_name} is outside its string table")
        end = strtab.find(b"\0", st_name) if st_name < len(strtab) else 0
        if end < 0:
            raise ElfError(f"{what}: symbol {i} name is not NUL-terminated")
        name = strtab[st_name:end].decode("ascii", "replace") if st_name < len(strtab) else ""
        symbols.append(Sym(name, st_info >> 4, st_info & 0xF, st_other & 0x3, st_shndx))
    return symbols


def parse_elf(data: bytes):
    """Validates an ELF shared object and returns
    dict(bits, machine, load_aligns, symbols, static_names, shnum, exports).
    Raises ElfError (never struct.error/IndexError) for anything malformed."""
    n = len(data)
    if n < 16 or data[:4] != b"\x7fELF":
        raise ElfError("not an ELF file")
    ei_class, ei_data = data[4], data[5]
    if ei_data != 1:
        raise ElfError("only little-endian ELF is supported")
    if ei_class == 2:
        bits, ehsize, phent_exp, shent_exp = 64, 64, 56, 64
    elif ei_class == 1:
        bits, ehsize, phent_exp, shent_exp = 32, 52, 32, 40
    else:
        raise ElfError(f"bad ELF class {ei_class}")
    if n < ehsize:
        raise ElfError("truncated ELF header")
    e_type, machine = _unpack("<HH", data, 0x10, "e_type/e_machine")
    if e_type != ET_DYN:
        raise ElfError(f"e_type {e_type} is not ET_DYN (not a shared object)")
    if bits == 64:
        e_phoff, e_shoff = _unpack("<QQ", data, 0x20, "e_phoff/e_shoff")
        e_phentsize, e_phnum, e_shentsize, e_shnum, _ = _unpack("<HHHHH", data, 0x36, "header table sizes")
    else:
        e_phoff, e_shoff = _unpack("<II", data, 0x1C, "e_phoff/e_shoff")
        e_phentsize, e_phnum, e_shentsize, e_shnum, _ = _unpack("<HHHHH", data, 0x2A, "header table sizes")

    if e_phnum == 0:
        raise ElfError("no program headers")
    if e_phentsize != phent_exp:
        raise ElfError(f"program header entry size {e_phentsize}, expected {phent_exp}")
    if e_phoff > n or e_phnum * e_phentsize > n - e_phoff:
        raise ElfError("program header table lies outside the file")
    if e_shnum == 0:
        raise ElfError("no section headers (cannot read the dynamic symbol table)")
    if e_shentsize != shent_exp:
        raise ElfError(f"section header entry size {e_shentsize}, expected {shent_exp}")
    if e_shoff > n or e_shnum * e_shentsize > n - e_shoff:
        raise ElfError("section header table lies outside the file")

    load_aligns = []
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        if bits == 64:
            p_type, _fl, p_offset, p_vaddr, _pa, p_filesz, p_memsz, p_align = _unpack("<IIQQQQQQ", data, off, f"program header {i}")
        else:
            p_type, p_offset, p_vaddr, _pa, p_filesz, p_memsz, _fl, p_align = _unpack("<IIIIIIII", data, off, f"program header {i}")
        if p_type != PT_LOAD:
            continue
        if p_align > 1 and p_align & (p_align - 1):
            raise ElfError(f"PT_LOAD {i}: p_align {p_align:#x} is not a power of two")
        if p_align > 1 and (p_vaddr - p_offset) % p_align:
            raise ElfError(f"PT_LOAD {i}: p_vaddr {p_vaddr:#x} and p_offset {p_offset:#x} are not congruent modulo p_align {p_align:#x}")
        if p_filesz > p_memsz:
            raise ElfError(f"PT_LOAD {i}: p_filesz {p_filesz:#x} is larger than p_memsz {p_memsz:#x}")
        if p_offset > n or p_filesz > n - p_offset:
            raise ElfError(f"PT_LOAD {i}: file range (offset {p_offset:#x}, size {p_filesz:#x}) lies outside the file")
        load_aligns.append(max(p_align, 1))
    if not load_aligns:
        raise ElfError("no PT_LOAD segment")

    sections = []
    for i in range(e_shnum):
        off = e_shoff + i * e_shentsize
        if bits == 64:
            _n, sh_type, _fl, _a, sh_off, sh_size, sh_link, _i, _al, sh_entsize = _unpack("<IIQQQQIIQQ", data, off, f"section header {i}")
        else:
            _n, sh_type, _fl, _a, sh_off, sh_size, sh_link, _i, _al, sh_entsize = _unpack("<IIIIIIIIII", data, off, f"section header {i}")
        sections.append((sh_type, sh_off, sh_size, sh_link, sh_entsize))

    dynsym = [s for s in sections if s[0] == SHT_DYNSYM]
    if not dynsym:
        raise ElfError("no .dynsym section (SHT_DYNSYM)")
    if len(dynsym) > 1:
        raise ElfError("multiple .dynsym sections")
    symbols = _read_symbols(data, dynsym[0], sections, bits, ".dynsym")

    static_names = set()
    for section in (s for s in sections if s[0] == SHT_SYMTAB):
        try:
            static_names |= {s.name for s in _read_symbols(data, section, sections, bits, ".symtab")}
        except ElfError:
            pass  # .symtab is only used to explain a missing export

    exports = {s.name for s in symbols if s.name and not symbol_problems(s, e_shnum)}
    return {"bits": bits, "machine": machine, "load_aligns": load_aligns, "symbols": symbols,
            "static_names": static_names, "shnum": e_shnum, "exports": exports}


def jni_export_errors(elf, expected):
    """Per expected JNI symbol: why it is missing or invalid (empty list = all good)."""
    errors = []
    for name in sorted(expected):
        found = [s for s in elf["symbols"] if s.name == name]
        if not found:
            where = "present only in .symtab, not in .dynsym" if name in elf["static_names"] else "missing from .dynsym"
            errors.append(f"JNI export {name}: {where}")
        elif not any(not symbol_problems(s, elf["shnum"]) for s in found):
            errors.append(f"JNI export {name}: not a valid export: " + "; ".join(symbol_problems(found[0], elf["shnum"])))
    return errors


def zip_data_offset(apk_path: Path, info: zipfile.ZipInfo) -> int:
    with open(apk_path, "rb") as f:
        f.seek(info.header_offset)
        header = f.read(30)
    if len(header) < 30 or header[:4] != b"PK\x03\x04":
        raise ElfError(f"bad local header for {info.filename}")
    name_len, extra_len = struct.unpack_from("<HH", header, 26)
    return info.header_offset + 30 + name_len + extra_len


def expected_jni_exports():
    import check_jni_symbols as jni  # same directory; main() only runs under __main__
    prefix = f"Java_{jni.mangle(jni.PACKAGE)}_{jni.CLASS}_"
    return {prefix + name for name in jni.kotlin_methods()}


def default_abis():
    props = (ROOT / "gradle.properties").read_text(encoding="utf-8")
    for line in props.splitlines():
        if line.startswith("rpx.rustAbis="):
            return [a.strip() for a in line.split("=", 1)[1].split(",") if a.strip()]
    return ["arm64-v8a", "x86_64"]


def check_apk(apk: Path, abis, require_16k, expected_exports, lib_name="librpx_core.so"):
    errors, report = [], []
    try:
        zf = zipfile.ZipFile(apk)
    except (OSError, zipfile.BadZipFile) as e:
        return [f"cannot read {apk}: {e}"], report
    with zf:
        names = {i.filename: i for i in zf.infolist()}
        for abi in abis:
            entry = f"lib/{abi}/{lib_name}"
            info = names.get(entry)
            if info is None:
                errors.append(f"{entry} is missing from the APK")
                continue
            needs_16k = abi in require_16k
            if info.compress_type != zipfile.ZIP_STORED:
                errors.append(f"{entry} is compressed in the zip (must be stored, uncompressed)")
            try:
                offset = zip_data_offset(apk, info)
                data = zf.read(info)
            except (ElfError, zipfile.BadZipFile, NotImplementedError, RuntimeError, EOFError, zlib.error, OSError) as e:
                errors.append(f"{entry}: cannot read the entry: {e}")
                continue
            if needs_16k and offset % PAGE_16K != 0:
                errors.append(f"{entry}: zip data offset {offset} is not a multiple of 16384")
            try:
                elf = parse_elf(data)
            except ElfError as e:
                errors.append(f"{entry}: malformed ELF: {e}")
                continue
            if abi in ABI_MACHINE:
                machine, bits = ABI_MACHINE[abi]
                if elf["machine"] != machine or elf["bits"] != bits:
                    errors.append(f"{entry}: ELF is machine {elf['machine']} / {elf['bits']}-bit, "
                                  f"but ABI {abi} needs machine {machine} / {bits}-bit")
            min_align = min(elf["load_aligns"])
            if needs_16k and min_align < PAGE_16K:
                errors.append(f"{entry}: PT_LOAD p_align {min_align:#x} < 0x4000 (not 16 KB aligned)")
            jni_errors = jni_export_errors(elf, expected_exports)
            errors.extend(f"{entry}: {e}" for e in jni_errors)
            report.append(f"{entry}: {elf['bits']}-bit, stored={info.compress_type == zipfile.ZIP_STORED}, "
                          f"zip offset%16384={offset % PAGE_16K}, min PT_LOAD align={min_align:#x}, "
                          f"JNI exports ok={not jni_errors} ({len(expected_exports)} expected)")
    return errors, report


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("apk", type=Path)
    p.add_argument("--abis", nargs="+", default=None)
    p.add_argument("--require-16k", nargs="*", default=["arm64-v8a", "x86_64"])
    a = p.parse_args(argv)
    abis = a.abis or default_abis()
    errors, report = check_apk(a.apk, abis, set(a.require_16k), expected_jni_exports())
    print("\n".join(report))
    if errors:
        print("FAILED:")
        print("\n".join(" - " + e for e in errors))
        return 1
    print(f"native libraries OK in {a.apk} for ABIs {abis}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
