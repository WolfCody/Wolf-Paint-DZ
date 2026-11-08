# Stage 2 plan: raster core

Status: **Stage 2.0 (hardening) DONE** (green CI run #7, commit 8e7ce0b, 2026-11-02). **Stage 2.1a (core-raster primitives) DONE** (host-only Rust, not wired into the app yet). Stages 2.1b and later are planned but not started.

Goal: on a phone, open a raster document (up to 8000 x 8000), draw and erase with a **stylus (primary)** or a finger (secondary), pan/zoom, undo/redo, use a few layers, save and reopen. Architecture stays Kotlin + Compose + Rust over JNI; `applicationId` stays `com.dzekiwolf.wolfpaintdz`; minSdk stays 26; no INTERNET permission, cloud, ads, analytics, telemetry or network dependencies. How that is enforced, and what the checks can and cannot prove: `docs/CI.md` ("What the offline checks do and do not prove").

Primary test device and its caveats (software-GLES property): `docs/DEVICES.md`.

## ADRs (decisions logged before they affect code)
1. **Pixel format:** RGBA8, premultiplied alpha, sRGB-encoded, blending in that space (stage 2: Normal only).
2. **Stroke buffer:** a stroke accumulates in a per-stroke coverage buffer with more than 8 bits (u16 or f32), committed to the layer with the stroke opacity on release. Prevents banding from soft dabs.
3. **Viewport:** Rust owns the view transform; Kotlin sends gesture deltas.
4. **Presenter:** CPU presenter first (`ANativeWindow` lock + blit); `wgpu` added behind a cargo feature and becomes default only after gate G1.
5. **Working copy:** a directory (tile files + change journal), temp-dir + rename. Stage 6 wraps it into the `.rpx` ZIP.
6. **Kotlin side:** no new Kotlin library until stage 2.7; two screens are routed by state.

## 2.0 Hardening (DONE)
Completed items listed in `docs/VERIFICATION.md`. CI run #7 (commit 8e7ce0b) is green: bootstrap, rust, android (debug + release), supply-chain. 16 Kotlin unit tests 0 failures.

Remaining items for the next commit:
* Run the emulator job once (`run_instrumented`) and record the result.
* Fill "to measure" cells in `docs/DEVICES.md` (Termux `getprop` list).
* Decide on blocking clippy (`-D warnings`) and add Android lint.
* Pin GitHub-owned actions by SHA if desired.

## 2.1a core-raster tile primitives (DONE in this commit — host-only, not wired into the app)
`crates/core-raster`:
* `Tile` (256 × 256 RGBA8 premultiplied, heap-allocated): `transparent`, `get`, `set`, `fill`, `row`, `is_transparent`, `from_bytes`, `premultiply`. 11 host tests.
* `TileStore` (sparse copy-on-write map): `pixel`, `set_pixel`, `tile_mut` (COW), `snapshot`, `prune_transparent`, `prune_all_transparent`, `insert`/`remove` (undo), `sorted_coords`, `resident_bytes`, `shared_tiles`. 17 host tests covering COW, transparency rules, 8000×8000 memory accounting (100 tiles = ~26 MB not 256 MB), thread-safety.
* geometry (`TileCoord`, `PixelRect`, `ClippedRect`, `TileRange`): `locate`, `origin`, `clip_to_canvas`, `covering`, `whole_canvas`, `coords`. 11 host tests including brute-force and 2000 random rectangles near tile edges.
Crate: no dependencies, `unsafe_code = "forbid"`, not yet in `core-ffi` or the JNI bridge.

CI additions in this commit:
* `check_apk_native.py` now checks all third-party native libs of 64-bit ABIs (8 new tests).
* `check_lock_change.py` + `test_check_lock_change.py` (10 tests).
* Bootstrap step `Cargo.lock matches the workspace crates` + `test_workflow_lock_step.py` (5 tests).
* SDK install step retries transient "Error on ZipFile" + `test_workflow_sdk_step.py` (9 tests). Previously this error aborted CI run #6.
* Real R8 artifacts used as test fixtures; 11 `RealArtifactTests`.
* Consistency script: TILE_SIZE parity between core-model and core-raster, crate hygiene, lock-step and SDK-retry guards.

## 2.1b Wire core-raster into the engine and JNI bridge (next)
Extend `core-model::Document` to own a list of `RasterLayer` (name, id, visible, opacity), each wrapping a `TileStore`. Expose through JNI: add/remove/reorder layers, `tilesAllocated` count in `DocumentInfo::to_wire`, `nativeSetPixel(handle, layer, x, y, r, g, b, a)` as a first drawing primitive. No GPU, no brush engine yet.
Gate: `cargo test --workspace` passes, the app compiles, the gallery can create a document and the debug-info row shows the correct tile count.

## 2.2 core-brush v0: dab, stroke, eraser
Round dab with hardness falloff, spacing, size/opacity/flow, linear pressure mapping, per-stroke buffer (ADR 2), eraser as destination-out, deterministic (seeded) randomness hook for later.

## 2.3 core-history v0: undo/redo
Per-stroke entry: the previous `Arc<Tile>` of every touched tile. Byte budget with LRU eviction.

## 2.4 CPU compositor and viewport
View transform, visible tile query, dirty-rect tracking, output to an RGBA8 buffer.

## 2.5 Surface and presenter (then GPU)
Step A: CPU presenter on `SurfaceView` / `ANativeWindow`. Step B: `wgpu` behind a cargo feature (gate G1: GPU output equals the CPU oracle ± 1 per channel on the emulator and the test device, plus rotation and background cycles).

## 2.6 Input and gestures (stylus first)
See input design in `docs/DEVICES.md`.

## 2.7 Editor UI

## 2.8 Memory budget and spill

## 2.9 Basic save/reopen

## 2.10 Acceptance and measurements (`docs/PERF.md`)

## Decisions needed from the maintainer
Answered: test device (`docs/DEVICES.md`), stylus primary, finger secondary, sRGB blending is acceptable for stage 2, wgpu deferred to gate G1.
Still open:
* Stylus model and whether it reports pressure/tilt (see input diagnostics in 2.6).
* Branch strategy (feature branch + PR per sub-stage).
