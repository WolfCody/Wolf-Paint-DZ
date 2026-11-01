# Test devices

Columns: **reported** = stated by the developer; **measured** = produced by this project's own tooling or logs. Nothing in the "reported" column has been verified by the project yet unless the note says so.

## Primary device
| Item | Value | Status |
|---|---|---|
| Manufacturer / model | Xiaomi 24069PC21G | reported |
| Android / API | 15 / 35 | reported |
| CPU ABI | arm64-v8a (aarch64) | reported; **measured**: the engine card shows `android aarch64, 64-bit` (screenshot of 2026-10-31) |
| GPU | Qualcomm Adreno | reported |
| EGL | Adreno | reported |
| Vulkan | Adreno | reported |
| OpenGL ES | 3.2 (system property) | reported |
| `persist.sys.force_sw_gles` | `1` | reported, **not interpreted yet** |
| Memory page size | unknown (4 KB expected on most Android 15 phones) | to measure |
| Stylus | primary input method; model, pressure/tilt support unknown | to measure |
| Touch | finger input is secondary | reported |

Also measured from the same screenshot: the gallery shows 2048 x 2048 as an 8 x 8 grid (64 tiles, 16.8 MB dense RGBA) and 8000 x 8000 as 32 x 32 (1024 tiles, 256 MB), matching the Rust numbers (2048 x 2048 x 4 = 16,777,216 B).

## The software-GLES property
`persist.sys.force_sw_gles=1` suggests that OpenGL ES may be rendered in software on this device, but what it does exactly depends on the ROM. Therefore **an Adreno entry in the device report is not proof of hardware acceleration**, and the project must not assume it:
* the renderer must report what it really got: `GL_RENDERER` / `GL_VENDOR` from an EGL context for the GL path, and `AdapterInfo` (name, backend, device type) from `wgpu` for the Vulkan path (stage 2.5);
* an adapter of type CPU, or a renderer such as llvmpipe / SwiftShader / "Android Emulator", counts as **software**; software results are never used for performance conclusions;
* Vulkan and GLES are separate drivers: software GLES does not imply software Vulkan. Both must be measured separately;
* the CPU presenter (stage 2.5, step A) is the baseline that works regardless of the answer.

## How to fill in the missing values (Termux on the phone, no root)
```bash
getprop ro.product.manufacturer; getprop ro.product.model
getprop ro.build.version.release; getprop ro.build.version.sdk
getprop ro.product.cpu.abilist
getprop persist.sys.force_sw_gles
getprop ro.hardware.egl; getprop ro.hardware.vulkan
getprop ro.opengles.version          # 196610 means OpenGL ES 3.2
getconf PAGESIZE
```
Stylus capabilities are measured by the input diagnostics planned in stage 2.6 (tool types, available axes, ranges, buttons); until then they stay "unknown".

## Other targets
| Target | Use |
|---|---|
| Android Emulator x86_64, API 34, `google_apis` (CI, opt-in) | instrumented tests; GPU is the emulator's software/host renderer, never a performance reference |
| Any 16 KB-page device or emulator image | verifies the 16 KB packaging (static checks already run on every APK) |
