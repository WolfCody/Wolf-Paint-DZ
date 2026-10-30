# RPX Paint architecture (stage 1)

## Decisions

1. **Truth lives in Rust.** Document model, tiles, brushes, history and project I/O are Rust. Kotlin is UI, lifecycle, SAF, settings.
2. **CPU model is the source of truth, GPU is a presenter.** Tiles (256x256 RGBA8, premultiplied, sparse map) live in RAM/disk on the CPU side. The GPU (wgpu) uploads visible tiles and composites layers for display. If a GPU context is lost, textures are rebuilt from CPU tiles. Blend math has one CPU reference implementation with golden tests; shaders are checked against it.
3. **Raster brushes: custom CPU stamping engine** (tile-local, multithreaded per tile). General 2D libraries do not give brush-specific accumulation, flow/opacity, smudge.
4. **Vector: own editable scene graph** (`core-vector`, stage 3). Rendering and export rasterization via tiny-skia into tiles. SVG: `roxmltree` + own mapper for the editable model; usvg/resvg only for rendering/validation, because usvg flattens shapes, `use` and styles and therefore cannot be an editing model. Unknown elements are preserved as raw XML.
5. **wgpu enters in stage 2 after a device spike**, not before: pinned version, `ANativeWindow` surface, Vulkan first with GLES fallback, and a CPU presenter (`ANativeWindow_lock`) as a safety net.

## Engine comparison

| Option | For | Against | Role |
|---|---|---|---|
| wgpu | Rust, Vulkan/GLES on Android, compute+render, `AndroidNdkWindowHandle` from `ANativeWindow` | API breaks roughly every release; shaders are WGSL (GLSL only through naga's limited front-end); GPU memory must be managed by us | display + compositing |
| Skia (skia-safe) | Mature paths/text/blend modes/SVG | Large native binary per ABI, C++ build/prebuilt binaries, no tile-aware canvas, adds a second GPU stack | rejected |
| tiny-skia | Pure Rust CPU, small, BSD-3, used by resvg | Whole-`Pixmap` model, no tiling, subset of Skia | per-tile vector/shape rasterization, export |
| Vello | Fast GPU vector scenes | Alpha (README says so), needs compute shaders, not a pixel-painting engine | possible later vector viewport; not on the critical path |
| Custom CPU | Exact brush semantics, deterministic tests | Must be written and optimized | brush engine, filters on tiles |
| GPU shaders | Cheap compositing/filters | Divergence from CPU math if untested | compositing, selected filters |

## Process/thread model

* JNI functions are thread-safe; shared state sits behind a registry mutex, per-document state will use finer locks.
* UI thread never calls heavy operations; the view model dispatches to `Dispatchers.Default`.
* Closing a document removes the handle immediately; running operations keep their `Arc` alive and finish safely.
* Stage 4+ input: pointer events are batched into one `FloatArray` per frame (x, y, pressure, tilt, orientation, time, pointerId, buttons), not one JNI call per event.
* Surface lifecycle (planned for stage 2): `surfaceCreated` -> Rust takes an `ANativeWindow` reference -> wgpu surface; `surfaceChanged` -> resize; `surfaceDestroyed` -> drop the wgpu surface and wait for the render thread **before** releasing the window.

## JNI contract (implemented)

| Kotlin (`RustBridge`) | Rust | Notes |
|---|---|---|
| `nativeInit(workDir)` | `init_engine` | idempotent, creates directory |
| `nativeVersion()` | `version_string` | |
| `nativeCreateDocument(name, mode, w, h): Long` | `Engine::create_document` | validated, returns opaque handle |
| `nativeDocumentInfo(h): LongArray` | `DocumentInfo::to_wire` | versioned layout, 10 longs |
| `nativeDocumentName(h)`, `nativeCloseDocument(h)`, `nativeOpenDocuments()` | | |
| `nativeDebugPanic()` | | proves panic containment |

Handles are `(generation << 32) | (slot+1)`, always a positive Java `long`; stale/forged values raise `InvalidHandle`.

Errors: Rust `RpxError` -> `ErrorKind` -> one Kotlin exception class per kind, thrown with `throw_new`. Panics are caught with `catch_unwind` and become `NativePanic`. `panic = "unwind"` is mandatory in Cargo profiles.

Real error-path code: `run`, `report`, `throw` in `crates/core-ffi/src/jni_bridge.rs`.

## Contradictions found in the spec and how they are resolved

1. **GLSL example vs wgpu.** wgpu's shading language is WGSL. Production shaders are WGSL. A GLSL ES listing may appear in docs as a reference translation only, not compiled into the app (the spec forbids unused demo shaders).
2. **ZIP-based `.rpx` vs incremental autosave.** ZIP is not append-friendly. Plan: working copy is a directory (tile files + change journal); `.rpx` is a verified ZIP snapshot written to a temp file and atomically renamed (through the SAF file descriptor for external files).
3. **Room vs `.rpx` as truth.** `.rpx` manifest is authoritative; Room is a rebuildable index.
4. **Timelapse with layer exclusion vs "no full-size frames in RAM".** History entries must carry stable layer ids and the brush engine must be deterministic (seeded RNG), so a timelapse can be replayed with a participation filter at reduced resolution. Decided in stage 7; constraint recorded now.
5. **No Compose Canvas/Bitmap for the main canvas** vs ZXing needing pixels: QR decoding on a temporary decoded bitmap is not the canvas; allowed.
6. **ibis Paint QR format is undocumented.** Needs real exported QR samples from you before any parser can be claimed.

## Roadmap

2 raster core + wgpu spike + drawing; 3 vector core + SVG; 4 layers/brushes/selection/history; 5 filters (>= 60, each with a test); 6 `.rpx`, Room, DataStore, autosave, gallery, export formats; 7 animation/timelapse; 8 measurements, memory limits, release. HUD customization, hotkeys/gestures, brush import (ABR via `brushkit`, ibis QR) are scheduled with stages 2-6 where their prerequisites exist.
