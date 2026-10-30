# Naming

**Product name: Wolf Paint DZ** (Android launcher label `app_name`, Gradle `rootProject.name = "wolf-paint-dz"`, CI artifact `wolf-paint-dz-debug-apk`, APK file `wolf-paint-dz-debug.apk`).

| Item | Value |
|---|---|
| `applicationId` | `com.dzekiwolf.wolfpaintdz` |
| `namespace` / Kotlin package | `com.dzekiwolf.wolfpaintdz` |
| JNI prefix | `Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_*` |
| Exception classes (looked up by Rust) | `com/dzekiwolf/wolfpaintdz/core/RpxNativeException$*` |

`applicationId` cannot be changed after the first publication to a store or after users have installed the app; decide before the first release. The namespace is bound to the JNI names in `crates/core-ffi`. Renaming it again means changing, together: `namespace` and `applicationId` in `app/build.gradle.kts`, the Kotlin source directories and `package` lines, `crates/core-ffi/src/{lib,jni_bridge}.rs`, `app/proguard-rules.pro`; then run `scripts/check_jni_symbols.py` and `scripts/check_project_consistency.py` (both derive the expected names from `namespace`).

## History
Until stage 1 the working title was "RPX Paint" (package `com.rpxpaint.app`). The old name appears **only in this file**; `check_project_consistency.py` fails if it appears anywhere else.

## Technical names that intentionally keep "rpx"
`rpx` is the name of the project file format and of the drawing engine, not of the product:

* `.rpx` project format (planned for stage 6)
* native library `rpx_core` (`librpx_core.so`, `System.loadLibrary("rpx_core")`)
* engine types `RpxError`, `RpxNativeException`
* build-script names `rpx.rustAbis`, `rpxNdkVersion`, `rpxRustAbis`
* log tags `rpx`, `rpx-core`; temp-directory prefixes in tests
* the phrase "RPX engine" in documentation
