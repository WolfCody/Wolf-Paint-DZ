# Verification log (Stage 2.1a: core-raster + CI hardening after CI runs #6/#7)

Authoring sandbox: Python 3.12, bash, OpenJDK 21 (runtime only), xmllint, PyYAML, zip. Network: only public GitHub pages. **Rust, Gradle, Android SDK/NDK, cargo-deny, aapt2, zipalign, emulator** — not available.

## Real CI artifact analysis (run #7, commit 8e7ce0b, artifacts uploaded by user)

| Artifact | Contents | Findings |
|---|---|---|
| `r8-mapping-release` | `mapping.txt` (5.19 MB, thousands of classes), `seeds.txt`, `usage.txt`, `configuration.txt` | Excerpts with the real R8 9.3.16 output are saved as test fixtures in `scripts/tests/fixtures/r8_real/`. Short class names (Kotlin lambda mangling: `$`, `$$inlined$`, `-O2vRcR0$`) confirmed. R8 writes only renamed/line-numbered members: our native methods are not in `mapping.txt` and that is correct — they are in `seeds.txt`. Constructor style is `ShortName(java.lang.String)` (e.g. `InvalidArgument(java.lang.String)`), not `<init>`. `usage.txt` lists removed members (`$stable`, `INSTANCE`, `<clinit>`) but no native method and no entire JNI class. The checker returned OK on the excerpts |
| `unit-test-reports` | Gradle HTML report, `testDebugUnitTest` | 16 Kotlin tests, 0 failures, 0 skipped, 0.869 s, 100 % successful (Gradle 9.5.0, 2026-11-02) |
| `wolf-paint-dz-debug-apk` | `wolf-paint-dz-debug.apk` (11.3 MB) | 2 of our libraries (`librpx_core.so` for arm64 and x86_64) + 2 third-party (`libandroidx.graphics.path.so`). All 4: stored (not compressed), zip offset 0 mod 16384, min PT_LOAD align 0x4000. JNI exports: all 8 `Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_*` present in `.dynsym` as GLOBAL DEFAULT FUNC defined symbols. INTERNET permission: not present (string pool scan). Package: `com.dzekiwolf.wolfpaintdz`. Launcher label: `Wolf Paint DZ`. Run #6 failure confirmed: CI run #7 (the run after the R8 fix) passed |

## Executed in this round (all in the sandbox)
| Check | Result |
|---|---|
| `python3 scripts/check_project_consistency.py --strict` (simulated placeholder tree) | OK |
| All Python tests: `python3 -m unittest discover -s scripts -p 'test_*.py' -v` | **137 tests OK** |
| `check_r8_keep.py` on the REAL excerpts (`scripts/tests/fixtures/r8_real/`) | OK; prints `R8 keeps OK: … 8 native methods …` |
| `check_apk_native.py` on the real debug APK | OK; also checks `libandroidx.graphics.path.so` |
| Mutation tests: R8 checker (16), APK checker (24), lock-change checker, SDK retry step, consistency script negative tests | every mutation caught by the targeted tests |
| YAML, TOML, XML, `bash -n`, `compileall`, JNI symbols | OK |

## NOT executed
`cargo test --workspace` (no Rust), including `core-raster` host tests. The first CI run with the new `core-raster` crate and the lock-step will reveal whether `cargo update --workspace` leaves the third-party packages identical.

## New this round
* Real R8 test fixtures + 11 `RealArtifactTests` covering the actual CI artifact.
* `check_apk_native.py`: now also checks all third-party native libraries of the 64-bit ABIs (storage, 16 KB alignment).
* `check_lock_change.py`: allows `cargo update --workspace` to touch only workspace crates; tested with a fake `cargo`.
* Bootstrap step `Cargo.lock matches the workspace crates`: detects a stale lock (new workspace member), runs `--workspace`-only update and verifies the diff; tested with `test_workflow_lock_step.py`.
* SDK retry step: retries transient "Error on ZipFile" failures (the actual error of CI run #7) with cleanup of partial downloads; tested with `test_workflow_sdk_step.py`.
* `crates/core-raster`: `Tile`, `TileStore`, geometry helpers — host-only, not wired into the app yet.
* Consistency script: TILE_SIZE parity, crate hygiene, lock-step and SDK-retry guards.

## Stage 2.1a status
Written and tested on the host. Not wired into the Android app or the JNI bridge yet (stage 2.1b).
