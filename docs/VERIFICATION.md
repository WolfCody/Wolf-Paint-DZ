# Verification log (stage 1, second fix pass)

Sandbox: Python 3.12, bash/dash, OpenJDK 21 (runtime only), xmllint, PyYAML, zip. **No** rustc/cargo, Gradle, Kotlin compiler, Android SDK/NDK, aapt2, PowerShell, emulator, or network (DNS fails).

## Executed
| Check | How | Result |
|---|---|---|
| JNI names/arity/types Kotlin vs Rust | `python3 scripts/check_jni_symbols.py` | OK (8 methods) |
| Project consistency (catalog refs, resources, ru/en, exceptions, offline rules, version agreement workflow<->project, workflow structure) | `python3 scripts/check_project_consistency.py` | OK; warns that wrapper/Cargo.lock are not committed |
| Same, strict mode | `... --strict` | **fails as designed** in this tree (wrapper and lock absent); CI runs it only after bootstrap |
| Negative tests of the checker | 6 injected defects (system gradle, lock generated in a job, upload without `always()`, version drift, ignored aapt2 failure, missing strict) | each one detected |
| `verify-apk-permissions.sh` logic | `scripts/test_verify_apk_permissions.sh` with a fake aapt2 | 10/10 pass (missing aapt2, missing/unreadable apk, failing aapt2, empty output, INTERNET in either dump, other/no permissions) |
| Regression proof for the old script | old script with a nonexistent aapt2 | printed "OK", exit 0 (false pass confirmed) |
| `generate-wrapper.sh` logic | mock `gradle`/`gradlew`, success + version-mismatch + no-gradle cases | as expected; JAR taken from the stage-2 (pinned) Gradle, mismatch aborts before copying |
| Version-match regex used in CI/scripts | bash | matches `Gradle 9.5.0`, rejects `9x5y0` and `9.5.01` |
| `bash -n` on all shell scripts, YAML parse of the workflow, XML/TOML parsing | | OK |

## NOT executed
`cargo test`, `cargo check` (host and Android), `cargo ndk`, any Gradle command (sync, `assembleDebug`, unit tests), instrumented tests, running `generate-wrapper.sh`/`.ps1` against a real Gradle (the bash one only against mocks; the PowerShell one not at all), running `aapt2` on a real APK, running the workflow on GitHub (YAML syntax only), `shellcheck`. **No APK has been built, so nothing is claimed about the built APK's contents or permissions.**
