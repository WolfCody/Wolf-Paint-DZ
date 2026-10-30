# Verification log (stage 1: compileSdk 37 fix + rename to Wolf Paint DZ)

Sandbox: Python 3.12, bash, OpenJDK 21 (runtime only), xmllint, PyYAML, zip. **No** rustc/cargo, Gradle, Kotlin compiler, Android SDK/NDK, `sdkmanager`, aapt2, PowerShell, emulator, or network.

## Executed
| Check | How | Result |
|---|---|---|
| SDK package id of API 37 | web sources (SDK Manager behaviour reported by dotnet/android-tools, Flutter's SDK tooling notes, published CI images: `platforms;android-37.0`; `android-37` is not published) | **`platforms;android-37.0`**. Not confirmed by running `sdkmanager` here |
| AGP/Gradle/JDK vs API 37 | official AGP 9.3 release notes and "about AGP" table | max API 37, min AGP for API 37.0 is 9.1.1, Gradle >= 9.5.0, JDK 17, Build Tools 36.0.0: **no upgrade needed** |
| SDK install step logic | workflow step extracted and run against a **mock** `sdkmanager` that reproduces "unknown package => warning, exit 0" | passes with `android-37.0`; fails with `android-37`; fails when another package is unknown |
| Project consistency incl. new checks (SDK package derived from `compileSdk`, plain `compileSdk = 37` rejected, targetSdk <= compileSdk, naming, namespace/JNI/proguard agreement, old-name hygiene) | `python3 scripts/check_project_consistency.py` | OK; negative tests detected: plain `compileSdk = 37`, wrong `PLATFORM`, `targetSdk` above `compileSdk`, old app name, old name in a Kotlin file, stale exception class prefix |
| Strict mode | `... --strict` | fails as designed here (wrapper + Cargo.lock absent); runs green only in CI after `bootstrap` |
| JNI names/arity/types (package now derived from `namespace`) | `python3 scripts/check_jni_symbols.py` | OK (8 methods); stale Rust prefix detected in a negative test |
| APK permission script logic | `scripts/test_verify_apk_permissions.sh` | 10/10 (fake aapt2) |
| YAML parse, `bash -n`, XML/TOML parse | | OK |

## NOT executed
`cargo test/check`, `cargo ndk`, any Gradle command (sync, `assembleDebug`, unit tests), instrumented tests, `sdkmanager` itself, `aapt2` on a real APK, the workflow on GitHub, `shellcheck`, PowerShell script. **No APK has been built.** Whether Gradle accepts `compileSdk { version = release(37) { minorApiLevel = 0 } }` with AGP 9.3.2 and whether Compose 1.12.1's AAR metadata is satisfied by 37.0 is documented behaviour but unproven here; the first CI run is the test. If `checkDebugAarMetadata` asks for 37.1, change `minorApiLevel` and `PLATFORM` together.
