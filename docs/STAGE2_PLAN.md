# Stage 2 plan: raster core

Status: **Stage 2.0 (hardening) is in progress** (checks and CI jobs are written, their first CI run is pending, see `docs/VERIFICATION.md`). Stages 2.1 and later are plan only; nothing in them is implemented.

Goal: on a phone, open a raster document (up to 8000 x 8000), draw and erase with a **stylus (primary)** or a finger (secondary), pan/zoom, undo/redo, use a few layers, save and reopen. Architecture stays Kotlin + Compose + Rust over JNI; `applicationId` stays `com.dzekiwolf.wolfpaintdz`; minSdk stays 26; no INTERNET permission, cloud, ads, analytics, telemetry or network dependencies. How that is enforced, and what the checks can and cannot prove: `docs/CI.md` ("What the offline checks do and do not prove").

Primary test device and its caveats (software-GLES property): `docs/DEVICES.md`.

Rules for every sub-stage: own commit/PR, CI green before the next one starts, tests written with the code, honest status (written / compiled / tested on device) in `docs/VERIFICATION.md`.

## Decisions to record first (short ADRs in `docs/adr/`)
1. **Pixel format:** RGBA8, premultiplied alpha, sRGB-encoded values, blending in that space (what paint apps do; Normal blend only in stage 2).
2. **Stroke precision:** a stroke is accumulated in a per-stroke coverage buffer with more than 8 bits (u16 or f32 per pixel), then committed into the layer with the stroke opacity. Reason: 8-bit accumulation of soft low-flow dabs bands and sticks; per-stroke buffering is also what makes "opacity" differ from "flow".
3. **Viewport ownership:** Rust owns the view transform (pan, zoom, rotation). Kotlin sends gesture deltas.
4. **Presenter strategy:** a CPU presenter (`ANativeWindow` lock + blit of the CPU-composited viewport) is built first and stays as fallback and as test oracle; `wgpu` is added behind a cargo feature and becomes default only after gate G1.
5. **Persistence:** stage 2 writes a *working-copy directory* (`manifest.json` + per-tile files with CRC32), written to a temp dir and renamed. It is the layout stage 6 wraps into the ZIP `.rpx`; it is not the final `.rpx`.
6. **Kotlin side:** no new libraries (no navigation library; two screens are routed by state in a ViewModel).

## 2.0 Hardening before features
Done by the Stage 2.0 patch (written; first CI run pending):
* `assembleRelease` (unsigned, R8) in CI, `verifyNoInternetRelease`, `aapt2` permission check on the release APK;
* `check_r8_keep.py`: `RustBridge` natives and `RpxNativeException` subclasses kept and not renamed (reads AGP's `seeds.txt`, `mapping.txt`, `usage.txt`);
* `check_apk_native.py` + `zipalign -c -P 16`: 16 KB zip alignment, ELF `PT_LOAD` alignment, uncompressed storage, JNI exports present, for the debug and the release APK;
* `deny.toml` + `supply-chain` job (cargo-deny 0.20.2: bans, licenses, sources; advisories informational) + offline `check_lock_policy.py`;
* third-party actions pinned to full SHAs, `cache-provider: basic`, online verification of the pins;
* opt-in emulator job reviewed and pinned (decision: stays manual, `docs/CI.md`);
* `docs/CI.md`, `docs/DEVICES.md`, `docs/VERIFICATION.md`.

Remaining Stage 2.0 tasks:
1. Push the patch and read the first run. Expected possible findings: R8 file formats, a license not yet in `allow`, release lint. Fix what the logs show, not what is guessed.
2. Run the emulator job once (`run_instrumented`) and record the result.
3. Fill the "to measure" cells of `docs/DEVICES.md` (Termux `getprop` list).
4. Decide on blocking clippy (`-D warnings`) and add Android lint to CI.
5. Pin GitHub-owned actions as well if wanted (SHAs must come from `git ls-remote`).
Gate: all CI jobs green on `main`, `check_project_consistency.py --strict` green.

## 2.1 `core-raster`: tiles and layers (host-only Rust)
`TileCoord`, `Tile` (256 x 256 RGBA8 premultiplied), `TileStore` (sparse map, `Arc<Tile>` copy-on-write), `RasterLayer` (id, name, visible, opacity), layer stack in `Document`, bounds clipping, memory accounting (bytes of resident tiles), transparent tiles are never stored.
Tests: tile addressing at canvas edges (non-multiple of 256), COW does not alter snapshots, empty tiles are not allocated, 8000 x 8000 document with 100 touched tiles holds about 100 x 256 KiB, not 256 MB.

## 2.2 `core-brush` v0: dab, stroke, eraser
Round dab with hardness falloff, spacing, size/opacity/flow, linear pressure mapping, distance-based resampling of input points, per-stroke buffer (ADR 2), commit to the layer, eraser as destination-out, deterministic (seeded) randomness hook for later.
Tests: exact pixel values of one dab, strokes crossing tile borders without seams, opacity cap holds when a stroke overlaps itself, eraser on transparent pixels, same input gives identical bytes.

## 2.3 `core-history` v0: undo/redo
One entry per stroke: the *previous* `Arc<Tile>` of every tile touched (cheap because of COW). Undo/redo swap tiles; redo is cleared by a new action; byte budget with oldest-entry eviction; grouping hook.
Tests: undo/redo across tile borders, budget eviction keeps the document valid, redo invalidation, memory returns after eviction.

## 2.4 `core-render`: CPU compositor and viewport
View transform, visible tile query, nearest/bilinear sampling, checkerboard, layer visibility and opacity, dirty-rect tracking, output to an RGBA buffer. Known limitation recorded: no tile pyramid yet, so heavy zoom-out aliases.
Tests: golden images for pan/zoom/rotation corner cases; this compositor is the oracle for the GPU path.

## 2.5 Surface, presenter, then GPU (two steps)
Step A, CPU presenter: `AndroidView(SurfaceView)` inside the editor screen; JNI `surfaceCreated/Changed/Destroyed`; a Rust render thread that owns the `ANativeWindow` reference; frames only when something is dirty. Shutdown order on `surfaceDestroyed`: stop frames, wait for the thread, release the window. Rotation, background/foreground and surface re-creation tested on the emulator.
Step B, `wgpu` spike (feature `gpu`): pin the version that is current at that time and re-check its API, MSRV, Android backends and features; disable unneeded default features; review its dependency tree for network-capable crates (`cargo tree`), because the deny-list cannot prove their absence; Vulkan first, GLES fallback; surface create/resize/destroy, device-lost recovery rebuilding textures from CPU tiles; visible tiles in an atlas (an 8000 px canvas never becomes one texture); WGSL compositing shader.
Renderer honesty (device caveat, `docs/DEVICES.md`): log what was really obtained: EGL `GL_RENDERER`/`GL_VENDOR` for GL, `wgpu` `AdapterInfo` (name, backend, device type) for Vulkan; show it in a debug screen. A CPU-type adapter or a software renderer string (llvmpipe, SwiftShader, Android Emulator) is classified as software and never used for performance conclusions; GLES and Vulkan are classified separately.
Gate G1: GPU output equals the CPU oracle within +-1 per channel (readback test on emulator and the maintainer's phone) and survives rotation and background cycles; otherwise the CPU presenter stays default and the GPU work moves out of stage 2.

## 2.6 Input and gestures (stylus first, finger second)
Principles: use only what Android reports; never invent values; do not tie the code to one stylus vendor.
* **Tool type** per pointer from `MotionEvent.getToolType`: stylus, eraser end, finger, mouse; stylus and finger are separate input paths with separate settings (finger draws / pans / ignored; palm rejection: finger ignored while a stylus is in range, configurable).
* **Capabilities are detected, not assumed**: for the device, query `InputDevice.getMotionRange` for pressure, tilt, orientation, distance; a stylus without pressure is a normal case. The packed event carries a capability mask; absent axes are *absent* (flag cleared), not zero or a made-up default. An explicit user setting may *simulate* pressure from speed; it is off by default and labelled as simulation.
* **Per sample**: x, y, time, pointer id, tool type, pressure, tilt, orientation, hover distance, button state (stylus buttons, eraser end) when available.
* **Smooth strokes**: all historical samples of a batched `MotionEvent` (`getHistorical*`) are forwarded, in order, in one packed `FloatArray` per touch event, one JNI call.
* **Hover** (`ACTION_HOVER_*`) is delivered for the brush cursor when the stylus reports it.
* **Cancellation**: `ACTION_CANCEL`, window focus loss, surface destroyed, and a second finger landing mid-stroke end or cancel the stroke deterministically; a cancelled stroke is rolled back, never half-committed.
* One finger draws or pans according to setting; two fingers pan/zoom/rotate; undo/redo gestures go through the shared command dispatcher only.
* **Input diagnostics screen** (debug): shows tool types seen, available axes with their ranges, buttons, sample rate, so `docs/DEVICES.md` can be filled from facts.
Tests: pure Kotlin tests for batching, capability masks, arbitration and cancellation; emulator test with injected `MotionEvent`s (finger, stylus tool type, historical points, `ACTION_CANCEL`).

## 2.7 Editor UI (Compose, ru + en)
Gallery -> editor routing, brush/eraser, size and opacity sliders, color picker (HSV plus hex), undo/redo, minimal layer panel (add, delete, reorder, visibility, opacity, active layer). All actions go through one `EditorCommand` dispatcher so hotkeys and quick actions can reuse it later. No customizable HUD yet.

## 2.8 Memory budget and spill to disk
Budget for resident tiles, LRU eviction of clean tiles not used by the current stroke, spill files under the engine work dir, reload on access, counters exposed through JNI (`resident tiles`, `spilled tiles`, bytes). Tests: 8000 x 8000, several layers, strokes all over the canvas, memory stays under the budget, pixels identical after spill and reload.

## 2.9 Basic save and reopen
Working-copy format from ADR 5 with `schema_version`; validation on load (size limits, no path escape, corrupt tile reported instead of crash); temp-dir-plus-rename write; fault-injection test for an interrupted save; reopen from the gallery. New Rust dependencies expected here: `serde`, `serde_json`, `crc32fast`. Each one is checked (version, license, MSRV, Android build in CI) and reviewed for network or socket code before adding; the deny-list cannot prove their absence (`docs/CI.md`).

## 2.10 Acceptance and measurements
Scenario: create 8000 x 8000, 3 layers, strokes in different areas, pan/zoom, hide/show layers, undo/redo, save, close, reopen, compare a hash of the composite before and after. Record in `docs/PERF.md`: frame time (p50/p95) while drawing, input-to-pixel latency estimate, native heap and PSS, tile counts, save/load time, per device. Numbers are measured, never promised.

## Out of scope for stage 2
Other brush types, stabilization settings UI, selection, filters, blend modes other than Normal, adjustment layers, text, vector mode, final `.rpx` ZIP, Room, autosave, animation, brush import, hotkeys, HUD editor, GPU brush stamping.

## Main risks
| Risk | Mitigation |
|---|---|
| wgpu surface/driver problems on some devices | CPU presenter first, gate G1, GLES fallback, device list |
| Input latency and Compose/SurfaceView touch handling | touch handled on the `SurfaceView`, batched JNI calls, unbuffered dispatch to be evaluated and measured |
| Banding/sticking of soft strokes | per-stroke buffer with more than 8 bits (ADR 2) |
| History pins many tiles | byte budget, spill of history tiles in 2.8 |
| Sparse-tile bugs at canvas edges and tile seams | host tests first, golden images |
| A new dependency brings network capability | the `cargo-deny` deny-list only catches **known** crates; therefore every new dependency is reviewed before it is added (manifest, features, socket/TLS code) and the review is recorded; the other offline checks stay in force (`docs/CI.md`) |
| API drift in wgpu | exact pin, re-check docs at spike time |

## Decisions needed from the maintainer
Answered: test device (`docs/DEVICES.md`), stylus is the primary input, finger secondary.
Still open:
1. Stylus model and whether it reports pressure/tilt (answered by the input diagnostics screen, or tell me).
2. Minimum acceptable drawing smoothness on that phone (a target to measure against, not a promise).
3. Is sRGB-space blending acceptable for stage 2 (linear-light blending would be a later opt-in)?
4. Branch strategy: feature branch plus pull request per sub-stage?
