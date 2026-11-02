# Verification log (Stage 2.0, R8 check fix after CI run #6)

Authoring sandbox: Python 3.12, bash, git, OpenJDK 21 (runtime only), xmllint, PyYAML, zip, readelf. **No** rustc/cargo, Gradle, Kotlin compiler, Android SDK/NDK, aapt2, zipalign, cargo-deny, emulator, PowerShell. Network: only public GitHub pages could be fetched, not artifacts or logs.

## What was and was not available from CI run #6 (commit efa5f70)
| Source | Readable by the author? | What was learned |
|---|---|---|
| Run page `actions/runs/41290489356` | yes (public) | status Failure; exactly one error annotation: job "Wolf Paint DZ debug APK", **step 22** ("Process completed with exit code 1"); the four artifacts and their sizes |
| Step 22 = `check_r8_keep.py` | derived from the workflow step order | the R8 check is the step that failed. Steps 18 to 21 ran before it, so the release build, the release aapt2 permission check, `zipalign -c -P 16` and `check_apk_native.py` on the release APK passed (inferred; their log lines were not seen) |
| Jobs bootstrap, rust, supply-chain | no error annotation on the run page | inferred as passed; not confirmed by logs |
| Artifacts `bootstrap-files`, `r8-mapping-release`, `unit-test-reports`, `wolf-paint-dz-debug-apk` | **no** (download needs a GitHub login; nothing was uploaded to the sandbox) | contents unknown. The maintainer's description is the only source about the real R8 files |
| Step logs | no (login) | the exact error text of step 22 is unknown |

## Cause of the R8 failure
Reproduced with the previous checker (efa5f70) on a fixture built from the fragments the maintainer quoted (fixture seeds were incomplete, so only these two findings are valid evidence):
1. **Class names:** the class-line regex accepted only `[\w.$]`; a real mapping line such as `androidx.compose.foundation.ClickableKt$clickable-O2vRcR0$$inlined$clickableWithIndicationIfNeeded$1 -> dh:` was reported as "unrecognized line outside any class block".
2. **Wrong rule:** the checker demanded an entry in `mapping.txt` for every native method. R8 lists only renamed (or line-numbered) members, so unchanged native methods are absent. The correct evidence that they are kept is `seeds.txt`.
Other causes cannot be excluded without the step log (for example how R8 prints the exception constructors in `seeds.txt`).

## Executed (real results)
| Check | Command | Result |
|---|---|---|
| All Python tests, both discovery forms | `python3 -m unittest discover -s scripts -p 'test_*.py' -v` and `-s scripts/tests` | **117 tests OK** (was 91). Without the new `scripts/tests/__init__.py` the first form ran **0 tests** |
| New R8 tests | 50 tests: verbatim real fragments, hyphen/`$` class names, unchanged methods absent from mapping, renamed method (`nativeInit -> a`, each of the 8), renamed/missing `RustBridge`, missing method in seeds, truncated/garbage/empty files, exception constructors in 4 print styles, usage cases, CRLF, 60,000-class mapping in about 0.3 s | OK |
| Mutation testing of the R8 checker | 16 mutations (old class regex, ignore renames, ignore signatures, ignore class renames/missing classes, ignore seeds checks, ignore corrupt lines, ignore empty files, only `<init>`, usage checks, the old "entry required" rule) | each caught by the targeted tests (one invalid mutation of mine was detected as a no-op and redone) |
| Project consistency, strict, on a copy with placeholder wrapper/lock | `python3 scripts/check_project_consistency.py --strict` | OK |
| JNI symbols | `python3 scripts/check_jni_symbols.py` | OK (8 methods) |
| Whitespace/conflict markers | `git diff --check` in a temporary git repository (base = the efa5f70 tree incl. the maintainer's `.gitignore`) | no findings |
| Action pins (structure), `compileall`, YAML, TOML, XML, APK-permission script logic | | OK |

## NOT executed
Gradle and any Android build (**the corrected checker has not run on a real `assembleRelease` output**), the real `mapping.txt`/`seeds.txt`/`usage.txt` of run #6, the CI logs, `cargo` anything, `cargo deny`, the online SHA check, the emulator job, the workflow itself.

## Still assumed (decided by the next CI run)
1. How R8 9.3.16 prints the constructors of `RpxNativeException$*` in `seeds.txt`: the check accepts `<init>(java.lang.String)` and the ProGuard form `ShortName(java.lang.String)` and prints the real lines if neither matches.
2. The layout of `usage.txt` (only class-level and RustBridge-member removals are looked for).
3. Everything listed as open in the previous verification log for cargo-deny, licenses and release lint (no log shows them failing).
