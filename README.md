# Wolf Paint DZ (stage 1: skeleton with a Kotlin <-> Rust bridge)

Offline drawing app for Android: Kotlin + Jetpack Compose UI, graphics logic in Rust.
Raster and vector document modes exist as *configuration*; **there is no drawing yet**.

## What stage 1 contains
* Gallery: engine status/version from Rust, create raster/vector documents (validated up to 32768 px per side, e.g. 8000 x 8000 without allocating pixels), tile-grid facts computed in Rust, close documents.
* Rust: canvas model, tile-grid math, generational opaque handles, panic-safe JNI, typed Java exceptions.
* Build: `cargo ndk` wired into Gradle; the build fails if the merged manifest requests INTERNET.

## Not implemented yet
Stage 2 (raster core) is planned in `docs/STAGE2_PLAN.md`; Stage 2.0 hardening is in progress. Not implemented yet: tiles/pixel storage, brushes, GPU canvas (wgpu), layers, undo/redo, vector objects, SVG, `.rpx`, Room, autosave, filters, animation, hotkeys, HUD editor, brush import. Documents live only while the process runs. The vector mode is only a document mode flag at this stage; it must become a real editable object model (stage 3), never a raster canvas.

## Reproducible build
`gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar` and `Cargo.lock` are committed (commit 35412ef), so builds use the pinned Gradle 9.5.0 and pinned crate versions; inside Gradle `cargo` runs with `--locked`. Check on your clone: `git ls-files gradlew Cargo.lock gradle/wrapper/gradle-wrapper.jar`.

CI (`Wolf Paint DZ CI`, details in `docs/CI.md`): the `bootstrap` job reuses the committed files and publishes them as the artifact `bootstrap-files`; the other jobs download exactly that artifact. Other artifacts: `wolf-paint-dz-debug-apk` (the file `wolf-paint-dz-debug.apk`), `unit-test-reports`, `r8-mapping-release`. Emulator tests are opt-in (`run_instrumented`).

Regenerating the wrapper (for example after changing the Gradle version in `gradle-wrapper.properties`): `scripts/generate-wrapper.sh` (Linux/macOS/Git-Bash/Termux; needs any Gradle >= 8 and network) or `scripts\generate-wrapper.ps1` (Windows; not executed by the author). It works in two stages (installed Gradle writes the wrapper, then the generated `./gradlew wrapper` regenerates the JAR with the pinned Gradle), fails unless `./gradlew --version` reports exactly the pinned version, and prints the JAR's sha256 for comparison with gradle.org/release-checksums. Commit with `git add gradlew gradlew.bat gradle/wrapper/gradle-wrapper.jar && git update-index --chmod=+x gradlew`.

## Requirements (local)
Android Studio with AGP 9.3 support, JDK 17, Android SDK Platform 37.0 (sdkmanager package `platforms;android-37.0`; there is no `android-37`), Build Tools 36.0.0, NDK 28.2.13676358, rustup (the toolchain 1.99.1 and Android targets install from `rust-toolchain.toml`), `cargo install cargo-ndk --version 4.1.2 --locked`. Versions and reasons: `docs/VERSIONS.md`.

## Commands
```
python3 scripts/check_project_consistency.py --strict   # static checks, no toolchain needed
python3 scripts/check_jni_symbols.py
python3 scripts/check_lock_policy.py                    # Cargo.lock vs deny.toml (banned crates, sources)
python3 -m unittest discover -s scripts/tests           # tests of the check scripts
scripts/test_verify_apk_permissions.sh                  # logic test of the APK permission check (fake aapt2)
python3 scripts/verify_action_pins.py --online          # CI action pins match their release tags (git + network)
cargo test --workspace                                  # host Rust tests
./gradlew :app:testDebugUnitTest                        # JVM tests (Kotlin)
./gradlew :app:assembleDebug                            # cargo-ndk + manifest check + packaging
./gradlew :app:assembleRelease                          # unsigned, R8
python3 scripts/check_apk_native.py <apk>               # 16 KB alignment, stored .so, JNI exports
python3 scripts/check_r8_keep.py app/build/outputs/mapping/release
adb install -r app/build/outputs/apk/debug/app-debug.apk
./gradlew :app:connectedDebugAndroidTest                # emulator/device
```
Windows: use `.\gradlew.bat`. ABIs: `rpx.rustAbis` in `gradle.properties` (default `arm64-v8a,x86_64`; `armeabi-v7a` is opt-in).

## Release signing
```bash
keytool -genkeypair -v -keystore wolf-paint-dz-release.jks -alias wolfpaintdz -keyalg RSA -keysize 4096 -validity 10000
```
Create `keystore.properties` in the repo root (git-ignored): `storeFile=wolf-paint-dz-release.jks`, `storePassword=...`, `keyAlias=wolfpaintdz`, `keyPassword=...`. Then `./gradlew :app:assembleRelease`. Without the file the release APK is unsigned. R8 is on; JNI keep rules are in `app/proguard-rules.pro`. The legacy `isMinifyEnabled` DSL is used (AGP 9.3 documents it as still supported; the new `optimization {}` block is a later migration).

## Verifying there is no network access
1. Automatic: `verifyNoInternetDebug|Release` parses the **merged** manifest and fails on INTERNET (hooked into `assemble*`/`bundle*`).
2. Built APK (debug and release in CI): `scripts/verify-apk-permissions.sh <apk>`. It exits 0 only after `aapt2` read the APK manifest and INTERNET is absent; a missing `aapt2`, an unreadable APK or unexpected output is a failure. Alternative: `apkanalyzer manifest permissions <apk>`. CI runs the script with the real aapt2.
3. Source rules: `check_project_consistency.py` fails on any `uses-permission`, `allowBackup != false`, network/analytics dependencies and network APIs in Kotlin/Rust.
4. Runtime: airplane mode, create a document.

These checks make an accidental network feature hard to introduce; they do not prove that no library can open a socket. In particular `deny.toml` forbids a list of *known* network, TLS and telemetry crates and says nothing about unlisted ones. What each layer can and cannot see, and the rule that every new dependency is reviewed: `docs/CI.md`.

## Layout
```
Cargo.toml, rust-toolchain.toml
crates/core-model    document model, handles, engine session (no unsafe, no deps)
crates/core-ffi      JNI exports -> librpx_core.so
app/                 Android app (Compose gallery, RustBridge, tests)
scripts/             static checks, APK/R8/ELF checks, wrapper generators (+ tests in scripts/tests)
.github/workflows/   android.yml
deny.toml            cargo-deny policy (bans, licenses, sources)
docs/                ARCHITECTURE.md, VERSIONS.md, CI.md, DEVICES.md, VERIFICATION.md, NAMING.md,
                     STAGE1_AUDIT.md, STAGE2_PLAN.md
```
`applicationId` may be changed freely. The Kotlin package `com.dzekiwolf.wolfpaintdz` is part of the JNI names in `crates/core-ffi`; rename both together and re-run `check_jni_symbols.py`.

## Naming
Product name, identifiers and the technical names that keep `rpx` (file format, engine): `docs/NAMING.md`.

## .rpx
Planned for stage 6; example manifests will come from the real writer and be tested there.

## Verification status
See `docs/VERIFICATION.md` for the exact list of what was and was not executed.
