# RPX Paint (stage 1: skeleton with a Kotlin <-> Rust bridge)

Offline drawing app for Android: Kotlin + Jetpack Compose UI, graphics logic in Rust.
Raster and vector document modes exist as *configuration*; **there is no drawing yet**.

## What stage 1 contains
* Gallery: engine status/version from Rust, create raster/vector documents (validated up to 32768 px per side, e.g. 8000 x 8000 without allocating pixels), tile-grid facts computed in Rust, close documents.
* Rust: canvas model, tile-grid math, generational opaque handles, panic-safe JNI, typed Java exceptions.
* Build: `cargo ndk` wired into Gradle; the build fails if the merged manifest requests INTERNET.

## Not implemented yet
Tiles/pixel storage, brushes, GPU canvas (wgpu), layers, undo/redo, vector objects, SVG, `.rpx`, Room, autosave, filters, animation, hotkeys, HUD editor, brush import. Documents live only while the process runs. The vector mode is only a document mode flag at this stage; it must become a real editable object model (stage 3), never a raster canvas.

## First reproducible build (read this first)
The repository does not yet contain `gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar` and `Cargo.lock`: they must be produced by the real tools, not written by hand.

**Option A: GitHub Actions (no local setup).** Push the repo, open Actions -> `android` -> Run workflow. The independent `bootstrap` job reuses or generates `Cargo.lock` and the Gradle wrapper and uploads them as the artifact **`bootstrap-files`** *before* any test or build can fail. The `rust` and `android` jobs download exactly that artifact (they never resolve dependencies or generate the wrapper on their own), run `check_project_consistency.py --strict`, and the build goes through `./gradlew`. Other artifacts: `rpx-paint-debug-apk`, `unit-test-reports`. Tick `run_instrumented` for emulator tests (slow). Unpack `bootstrap-files` into the repo root, then `chmod +x gradlew` (artifacts drop the executable bit) and commit: `git add --chmod=+x gradlew gradlew.bat gradle/wrapper/gradle-wrapper.jar Cargo.lock`.

**Option B: locally.**
```bash
cargo generate-lockfile                 # creates Cargo.lock, commit it
scripts/generate-wrapper.sh             # Linux/macOS/Git-Bash; any Gradle >= 8 on PATH, network needed
```
The script works in two stages (installed Gradle writes the wrapper, then the generated `./gradlew wrapper` regenerates the JAR with the pinned Gradle) and fails unless `./gradlew --version` reports exactly the pinned version; it prints the JAR's sha256 so you can compare it with gradle.org/release-checksums. Windows PowerShell: `.\scripts\generate-wrapper.ps1` (same logic, not executed by the author). Then commit `gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar` (on Git: `git update-index --chmod=+x gradlew`). Optional hardening: add `distributionSha256Sum` for Gradle 9.5.0 from gradle.org/release-checksums.

Once `Cargo.lock` is committed, the Gradle task passes `--locked` to cargo.

## Requirements (local)
Android Studio with AGP 9.3 support, JDK 17, Android SDK Platform 36, Build Tools 36.0.0, NDK 28.2.13676358, rustup (the toolchain 1.99.1 and Android targets install from `rust-toolchain.toml`), `cargo install cargo-ndk --version 4.1.2 --locked`. Versions and reasons: `docs/VERSIONS.md`.

## Commands
```bash
python3 scripts/check_project_consistency.py   # static checks, no toolchain needed (add --strict once wrapper and Cargo.lock exist)
scripts/test_verify_apk_permissions.sh          # logic test of the APK permission check (fake aapt2)
python3 scripts/check_jni_symbols.py
cargo test --workspace                          # host Rust tests
./gradlew :app:testDebugUnitTest                # JVM tests (Kotlin)
./gradlew :app:assembleDebug                    # runs cargo-ndk + manifest check + packaging
adb install -r app/build/outputs/apk/debug/app-debug.apk
./gradlew :app:connectedDebugAndroidTest        # emulator/device
```
Windows: use `.\gradlew.bat`. ABIs: `rpx.rustAbis` in `gradle.properties` (default `arm64-v8a,x86_64`; `armeabi-v7a` is opt-in).

## Release signing
```bash
keytool -genkeypair -v -keystore rpx-release.jks -alias rpx -keyalg RSA -keysize 4096 -validity 10000
```
Create `keystore.properties` in the repo root (git-ignored): `storeFile=rpx-release.jks`, `storePassword=...`, `keyAlias=rpx`, `keyPassword=...`. Then `./gradlew :app:assembleRelease`. Without the file the release APK is unsigned. R8 is on; JNI keep rules are in `app/proguard-rules.pro`. The legacy `isMinifyEnabled` DSL is used (AGP 9.3 documents it as still supported; the new `optimization {}` block is a later migration).

## Verifying there is no network access
1. Automatic: `verifyNoInternetDebug|Release` parses the **merged** manifest and fails on INTERNET (hooked into `assemble*`/`bundle*`).
2. Built APK: `scripts/verify-apk-permissions.sh <apk>`. It exits 0 only after `aapt2` read the APK manifest and INTERNET is absent; a missing `aapt2`, an unreadable APK or unexpected output is a failure. Alternative: `apkanalyzer manifest permissions <apk>`. CI runs the script with the real aapt2.
3. Source rules: `check_project_consistency.py` fails on any `uses-permission`, `allowBackup != false`, network/analytics dependencies and network APIs in Kotlin/Rust.
4. Runtime: airplane mode, create a document.

## Layout
```
Cargo.toml, rust-toolchain.toml
crates/core-model    document model, handles, engine session (no unsafe, no deps)
crates/core-ffi      JNI exports -> librpx_core.so
app/                 Android app (Compose gallery, RustBridge, tests)
scripts/             static checks, wrapper generators, APK permission check
.github/workflows/   android.yml
docs/                ARCHITECTURE.md, VERSIONS.md
```
`applicationId` may be changed freely. The Kotlin package `com.rpxpaint.app` is part of the JNI names in `crates/core-ffi`; rename both together and re-run `check_jni_symbols.py`.

## .rpx
Planned for stage 6; example manifests will come from the real writer and be tested there.

## Verification status
See `docs/VERIFICATION.md` for the exact list of what was and was not executed.
