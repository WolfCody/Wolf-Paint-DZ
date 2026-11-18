#![allow(non_snake_case)]
use jni::objects::{JClass, JFloatArray, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, jlongArray, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use core_model::{CanvasSize, DocumentMode, Handle, RpxError};
use core_brush::BrushKind;
use crate::{catch_panic, engine, exception_class, exception_class_for, init_engine, version_string};
use crate::drawing_session::DrawingSession;
use crate::session_registry as sessions;

// ── plumbing ──────────────────────────────────────────────────────────────────
enum BE { Rpx(RpxError), Jni(jni::errors::Error) }
impl From<RpxError> for BE { fn from(e: RpxError) -> Self { BE::Rpx(e) } }
impl From<jni::errors::Error> for BE { fn from(e: jni::errors::Error) -> Self { BE::Jni(e) } }
type BR<T> = Result<T, BE>;

fn run<'l, T>(env: &mut JNIEnv<'l>, fb: T, body: impl FnOnce(&mut JNIEnv<'l>) -> BR<T>) -> T {
    match catch_panic(|| body(env)) {
        Ok(Ok(v)) => v,
        Ok(Err(BE::Rpx(e))) => { throw(env, exception_class_for(e.kind()), &e.to_string()); fb }
        Ok(Err(BE::Jni(jni::errors::Error::JavaException))) => fb,
        Ok(Err(BE::Jni(e))) => { throw(env, exception_class::INTERNAL, &e.to_string()); fb }
        Err(m) => { throw(env, exception_class::NATIVE_PANIC, &format!("panic: {m}")); fb }
    }
}
fn throw(env: &mut JNIEnv<'_>, cls: &str, msg: &str) {
    if env.exception_check().unwrap_or(false) { return; }
    let _ = env.throw_new(cls, msg);
}
fn rs(env: &mut JNIEnv<'_>, s: &JString<'_>) -> BR<String> { Ok(env.get_string(s)?.into()) }
fn hnd(raw: jlong) -> BR<Handle> {
    Handle::from_raw(raw as u64).ok_or_else(|| BE::Rpx(RpxError::InvalidHandle(raw as u64)))
}

// ── existing methods ──────────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeInit<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, d: JString<'l>) {
    run(&mut env, (), |e| { init_engine(&rs(e, &d)?)?; sessions::init(); Ok(()) })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeVersion<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>) -> jstring {
    run(&mut env, std::ptr::null_mut(), |e| Ok(e.new_string(version_string())?.into_raw()))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeCreateDocument<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, name: JString<'l>, mode: jint, w: jint, h: jint) -> jlong {
    run(&mut env, 0, |e| {
        let n = rs(e, &name)?;
        let m = DocumentMode::from_wire(mode)?;
        let c = CanvasSize::from_signed(w, h)?;
        let handle = engine()?.create_document(&n, m, c)?;
        if matches!(m, DocumentMode::Raster) {
            sessions::insert(handle, DrawingSession::new(c.width(), c.height()));
        }
        Ok(handle.raw() as jlong)
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeDocumentInfo<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) -> jlongArray {
    run(&mut env, std::ptr::null_mut(), |e| {
        let h = hnd(handle)?;
        let doc = engine()?.document(h)?;
        let mut wire = doc.info().to_wire();
        if let Some(n) = sessions::with(h, |s| s.tiles_allocated() as i64) { wire[8] = n; }
        let arr = e.new_long_array(wire.len() as jint)?;
        e.set_long_array_region(&arr, 0, &wire)?;
        Ok(arr.into_raw())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeDocumentName<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) -> jstring {
    run(&mut env, std::ptr::null_mut(), |e| {
        Ok(e.new_string(engine()?.document(hnd(handle)?)?.name())?.into_raw())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeCloseDocument<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) {
    run(&mut env, (), |_| { let h = hnd(handle)?; sessions::remove(h); engine()?.close_document(h)?; Ok(()) })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeOpenDocuments<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>) -> jlongArray {
    run(&mut env, std::ptr::null_mut(), |e| {
        let v: Vec<jlong> = engine()?.open_documents().into_iter().map(|h| h.raw() as jlong).collect();
        let arr = e.new_long_array(v.len() as jint)?;
        e.set_long_array_region(&arr, 0, &v)?;
        Ok(arr.into_raw())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeDebugPanic<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>) {
    run(&mut env, (), |_| {
        if cfg!(debug_assertions) { panic!("intentional debug panic"); }
        Err(BE::Rpx(RpxError::Unsupported("debug only")))
    })
}

// ── drawing methods ───────────────────────────────────────────────────────────
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeAddLayer<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, _name: JString<'l>) -> jlong {
    run(&mut env, -1, |_| {
        let h = hnd(handle)?;
        Ok(sessions::with(h, |s| {
            let lid = core_model::raster_layer::LayerId(s.layer_order.len() as u64 + 1);
            s.add_layer(lid);
            lid.0 as jlong
        }).unwrap_or(-1))
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeRemoveLayer<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, layer_id: jlong) {
    run(&mut env, (), |_| {
        let h = hnd(handle)?;
        sessions::with(h, |s| s.remove_layer(core_model::raster_layer::LayerId(layer_id as u64)));
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeSetActiveLayer<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, layer_id: jlong) {
    run(&mut env, (), |_| {
        sessions::with(hnd(handle)?, |s| {
            s.active_layer = Some(core_model::raster_layer::LayerId(layer_id as u64));
        });
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeSetLayerVisible<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, layer_id: jlong, visible: jboolean) {
    run(&mut env, (), |_| {
        sessions::with(hnd(handle)?, |s| {
            s.layer_visible.insert(core_model::raster_layer::LayerId(layer_id as u64), visible == JNI_TRUE);
        });
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeSetLayerOpacity<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, layer_id: jlong, opacity: jfloat) {
    run(&mut env, (), |_| {
        sessions::with(hnd(handle)?, |s| {
            s.layer_opacity.insert(core_model::raster_layer::LayerId(layer_id as u64), opacity);
        });
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeBrushSet<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>,
    handle: jlong, size: jfloat, hardness: jfloat, opacity: jfloat, flow: jfloat,
    r: jint, g: jint, b: jint, eraser: jboolean) {
    run(&mut env, (), |_| {
        sessions::with(hnd(handle)?, |s| {
            s.brush.size = size; s.brush.hardness = hardness;
            s.brush.opacity = opacity; s.brush.flow = flow;
            s.brush.color = [r as u8, g as u8, b as u8, 255];
            s.brush.kind = if eraser == JNI_TRUE { BrushKind::Eraser } else { BrushKind::Round };
        });
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeStrokeBegin<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, x: jfloat, y: jfloat, pressure: jfloat) {
    run(&mut env, (), |_| { sessions::with(hnd(handle)?, |s| s.stroke_begin(x, y, pressure)); Ok(()) })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeStrokePoints<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, pts: JFloatArray<'l>) {
    run(&mut env, (), |e| {
        let h = hnd(handle)?;
        let len = e.get_array_length(&pts)? as usize;
        let mut buf = vec![0f32; len];
        e.get_float_array_region(&pts, 0, &mut buf)?;
        sessions::with(h, |s| {
            let mut i = 0;
            while i + 2 < buf.len() { s.stroke_move(buf[i], buf[i+1], buf[i+2]); i += 3; }
        });
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeStrokeEnd<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) {
    run(&mut env, (), |_| { sessions::with(hnd(handle)?, |s| s.stroke_end()); Ok(()) })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeStrokeCancel<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) {
    run(&mut env, (), |_| { sessions::with(hnd(handle)?, |s| s.stroke_cancel()); Ok(()) })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeUndo<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) -> jboolean {
    run(&mut env, JNI_FALSE, |_| {
        Ok(if sessions::with(hnd(handle)?, |s| s.undo()).unwrap_or(false) { JNI_TRUE } else { JNI_FALSE })
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeRedo<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong) -> jboolean {
    run(&mut env, JNI_FALSE, |_| {
        Ok(if sessions::with(hnd(handle)?, |s| s.redo()).unwrap_or(false) { JNI_TRUE } else { JNI_FALSE })
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeRenderToBuffer<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>, handle: jlong, sw: jint, sh: jint) -> jni::sys::jbyteArray {
    run(&mut env, std::ptr::null_mut(), |e| {
        let h = hnd(handle)?;
        let size = sw as usize * sh as usize * 4;
        let mut buf = vec![0u8; size];
        sessions::with(h, |s| {
            s.viewport.screen_w = sw as u32; s.viewport.screen_h = sh as u32;
            s.render_to_buffer(&mut buf);
        });
        let arr = e.new_byte_array(size as jint)?;
        // SAFETY: &[u8] reinterpreted as &[i8] — standard JNI idiom, same size
        let sbuf: &[i8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const i8, size) };
        e.set_byte_array_region(&arr, 0, sbuf)?;
        Ok(arr.into_raw())
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeViewportUpdate<'l>(
    mut env: JNIEnv<'l>, _: JClass<'l>,
    handle: jlong, pan_x: jfloat, pan_y: jfloat, zoom: jfloat, rot: jfloat, sw: jint, sh: jint) {
    run(&mut env, (), |_| {
        sessions::with(hnd(handle)?, |s| {
            s.viewport.pan_x = pan_x; s.viewport.pan_y = pan_y;
            s.viewport.zoom = zoom; s.viewport.rotation_deg = rot;
            s.viewport.screen_w = sw as u32; s.viewport.screen_h = sh as u32;
        });
        Ok(())
    })
}
