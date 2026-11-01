# Verification log (Stage 2.0 hardening, final)

Authoring sandbox: Python 3.12, bash, OpenJDK 21 (runtime only), xmllint, PyYAML, zip, readelf. **No** rustc/cargo, Gradle, Kotlin compiler, Android SDK/NDK, aapt2, zipalign, cargo-deny, emulator, PowerShell, or network (DNS fails). Everything below that needs those tools is **not executed** and waits for the first CI run.

## Executed (real results)
| Check | How | Result |
|---|---|---|
| Python tests of the check scripts | `python3 -m unittest discover -s scripts/tests` | **91 tests OK** (previous patch: 41; all 41 scenarios are kept: 18 unchanged, 23 in the two rewritten scripts kept or renamed) |
| ELF / APK checker (`check_apk_native.py`) | 49 tests: HIDDEN, INTERNAL, PROTECTED, WEAK, LOCAL, SHN_UNDEF, reserved/out-of-range section index, non-FUNC type, symbol absent from `.dynsym`, symbol only in `.symtab`, name must match exactly; 21 malformed-ELF cases (truncation, table bounds, entry sizes, section indices, string-table bounds, unterminated names, duplicate `.dynsym`, `ET_EXEC`, PT_LOAD alignment/congruence/size/bounds, missing PT_LOAD, wrong machine for the ABI, 32-bit ELF in a 64-bit ABI) | each case ends as an APK-check **error**, none as a traceback |
| Mutation testing of that checker | 23 mutations (one check disabled at a time, bytecode cache off) | every mutation is caught by exactly the tests that target it |
| ELF parser on **real** linker output | libz and libc of the sandbox vs `readelf` (PT_LOAD align 0x1000, `inflate`/`malloc` valid exports, machine 62) | matches |
| R8 keep checker (`check_r8_keep.py`) | 24 tests: `nativeInit -> a` (the case from the review), each native method renamed individually, missing method entry, wrong signature, malformed line in the RustBridge block, garbage outside blocks, missing/renamed class, seeds and usage cases, fail-closed on missing/unparsable files | pass/fail as expected; 8 mutations all caught |
| The review scenario on the **real** project expectations | mapping generated from the real Kotlin sources, `nativeInit -> a` and a removed method | reported as errors |
| Project consistency | `python3 scripts/check_project_consistency.py`; strict mode on a copy with placeholder wrapper/lock and a synthetic Cargo.lock | OK |
| Regression checks of the consistency script | toml-ordering bug in `deny.toml` (see below), setup-gradle tag, missing `cache-provider`, zipalign without `-P 16`, missing R8 check, blocking advisories, missing cargo-deny step, relaxed `unknown-registry`, toolchain drift | each detected |
| Action pins | `python3 scripts/verify_action_pins.py` (structure) + tests with a fake `git` for the online mode | OK; real online resolution not run |
| `verify-apk-permissions.sh` | `scripts/test_verify_apk_permissions.sh` (fake aapt2) | 10/10 |
| Offline-policy statements | consistency script requires the limits statements in `docs/CI.md` and in the `deny.toml` header | OK |
| `compileall`, YAML parse, `bash -n`, XML/TOML parse, JNI symbol check | | OK |
| Patch applied over the full Stage 1 tree (commit-35412ef state, including the existing `scripts/check_jni_symbols.py`) | copy of the previous tree + placeholders, patch unzipped over it, then the whole suite | identical tree, all checks OK |

A bug found and fixed while doing this: a first version of `deny.toml` put the new `[bans.std-replacements]` table *before* the `deny = [...]` key, which TOML then assigns to that sub-table (the ban list would have disappeared from `[bans]`). Fixed and covered by a consistency check.

## NOT executed
`cargo test`, `cargo check`, `cargo ndk`, any Gradle command (sync, `assembleDebug`, **`assembleRelease` with R8**), unit and instrumented Android tests, **`cargo install cargo-deny` and `cargo deny`** (version 0.20.2 and the whole `deny.toml`, including `licenses.private` and the `allow` list, are unconfirmed until CI), real `aapt2`/`zipalign`, the real `check_apk_native.py` / `check_r8_keep.py` on a CI-built APK, the online SHA verification (the two pinned SHAs come from public pull requests), the emulator job, the workflow on GitHub, `shellcheck`, the PowerShell script. **No release APK exists; nothing is claimed about its contents.**

## First CI run decides
1. `check_r8_keep.py` assumes R8 writes an entry for **every** native method of RustBridge in `mapping.txt`. If R8 omits unrenamed members, the check reports "no entry" although nothing is wrong. Then send the real `r8-mapping-release` artifact; the rule must be adjusted with a real file in hand (not weakened blindly).
2. `cargo install cargo-deny --version 0.20.2 --locked` and the `deny.toml` field names (`bans.std-replacements` is confirmed by the 0.20 docs page; `licenses.private`/`confidence-threshold` were not visible on the 0.20 pages).
3. A license outside `MIT`, `Apache-2.0`, `Unicode-3.0` in the dependency tree (the log names the crate).
4. `assembleRelease` may show lint or R8 findings that the debug build never did.
5. zipalign check assumes AGP aligns the unsigned release APK to 16 KB.
6. A wrong action SHA fails the job during setup: run `python3 scripts/verify_action_pins.py --online` before pushing.
