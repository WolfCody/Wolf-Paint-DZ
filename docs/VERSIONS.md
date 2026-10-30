# Toolchain versions (checked 2026-10-29)

| Component | Version | Why / source |
|---|---|---|
| Android Gradle Plugin | 9.3.2 | Stable 9.3 line (AGP release notes). Requires Gradle >= 9.5.0, JDK 17, supports up to API 37. |
| Gradle | 9.5.0 | Minimum and default for AGP 9.3. Kotlin 2.4.0 is documented as compatible with 9.5.0. Newer Gradle (9.8.x exists) is not needed. |
| Kotlin / Compose compiler plugin | 2.4.0 | Released June 2026, compatible with Gradle 9.5.0. The Compose compiler plugin version equals the Kotlin version. |
| Kotlin plugin for Android | not applied | AGP 9 has built-in Kotlin; applying `org.jetbrains.kotlin.android` is an error. |
| compileSdk / targetSdk / minSdk | 37 / 36 / 26 | AGP 9.3 supports up to 37; 36 chosen as the conservative stable level. minSdk 26 as required. |
| Build Tools | 36.0.0 | AGP 9.3 default. |
| NDK | 28.2.13676358 | AGP default. r28+ aligns ELF segments to 16 KB (cargo-ndk also passes `max-page-size=16384`). |
| JDK | 17 | AGP minimum; bytecode target 17. |
| Compose BOM | 2026.09.00 | Latest BOM I could confirm in the official docs. |
| activity-compose | 1.13.0 | Latest stable on the AndroidX release page. |
| lifecycle (viewmodel-compose, runtime-compose) | 2.10.0 | Version used by the official Compose setup docs. |
| kotlinx-coroutines | 1.11.0 | Latest stable (May 2026). |
| androidx.test ext-junit / runner | 1.3.0 / 1.7.0 | AndroidX test release page. |
| JUnit | 4.13.2 | |
| Rust | 1.99.1 (edition 2024, MSRV 1.85) | 1.99.0 had a C-ABI return-value bug on some x86-64 targets, fixed in 1.99.1. |
| cargo-ndk | 4.1.2 | MSRV 1.86 (fine with 1.99.1). Flags used: `-t`, `-P`, `-o`; passes `--locked` through to cargo. |
| jni crate | 0.21.1 | 0.22.x is current but is a breaking redesign (`EnvUnowned`/`Env`, new class-loading rules on Android). I verified the 0.21 API (`throw_new` signature) and the 0.22 migration is a planned task, not a hidden assumption. |
| log / android_logger | 0.4 / 0.15 | android_logger 0.15.1 API (`init_once`, `Config::with_max_level`, `with_tag`) checked. |

## Compatibility notes

* Rust's platform-support page says Rust supports "the most recent LTS NDK". AGP's default NDK (r28c, used here) is not necessarily the LTS; r28+ is what gives 16 KB alignment by default. If CI shows a linker problem with r28, the first fallback is the NDK r27 line (`rpxNdkVersion` in `app/build.gradle.kts` and `NDK_VERSION` in the workflow).
* `wgpu` and `tiny-skia` are **not dependencies in stage 1**, so their compatibility with Rust 1.99.1 / NDK 28 is not claimed. It is checked in stage 2 when they are added (pinned versions, `cargo ndk` build on CI).
* `jni` 0.21.1 has a tiny MSRV and no native code; `log` and `android_logger` 0.15 are pure Rust plus the Android log library from the NDK.

## CI action versions (GitHub Actions)

checkout v7, setup-java v5, setup-gradle v6 (`gradle-version` input installs Gradle on PATH; used only in the bootstrap job), upload-artifact v7, download-artifact v8, cache v6 (all seen as current majors in 2026 release notes). `reactivecircus/android-emulator-runner@v2` is used only for the optional emulator job and its current version was **not** re-checked.

## Build status

These versions have **not been proven to build together** by a command run in the authoring environment (no Gradle, SDK, Rust or network there). The CI workflow is the first real test.
