//! JNI exports for `com.dzekiwolf.wolfpaintdz.core.RustBridge`.
//!
//! Contract (keep in sync with RustBridge.kt; `scripts/check_jni_symbols.py` verifies names
//! and arity):
//!
//! * Every function is thread-safe and may be called from any Java thread.
//! * Failures are reported as Java exceptions (`RpxNativeException` subclasses), never as
//!   magic return values. After throwing, the returned value is a dummy and must be ignored
//!   (the JVM does not deliver it because the exception propagates).
//! * Rust panics are caught and reported as `RpxNativeException$NativePanic`.
//! * Documents are addressed by opaque `long` handles that are validated on every call.
//!
//! Function names follow the JNI mangling rule: `Java_` + package with `_` + `_` + class +
//! `_` + method. The package and class contain no underscores, so no `_1` escapes are needed.

#![allow(non_snake_case)]

use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jlongArray, jstring};
use jni::JNIEnv;

use core_model::{CanvasSize, DocumentMode, Handle, RpxError};

use crate::{catch_panic, engine, exception_class, exception_class_for, init_engine, version_string};

enum BridgeError {
    Rpx(RpxError),
    Jni(jni::errors::Error),
}

impl From<RpxError> for BridgeError {
    fn from(error: RpxError) -> Self {
        BridgeError::Rpx(error)
    }
}

impl From<jni::errors::Error> for BridgeError {
    fn from(error: jni::errors::Error) -> Self {
        BridgeError::Jni(error)
    }
}

type BridgeResult<T> = Result<T, BridgeError>;

/// Runs `body` with panic containment and converts every failure into a pending Java
/// exception. Returns `fallback` in the failure case.
fn run<'local, T>(
    env: &mut JNIEnv<'local>,
    fallback: T,
    body: impl FnOnce(&mut JNIEnv<'local>) -> BridgeResult<T>,
) -> T {
    match catch_panic(|| body(&mut *env)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            report(env, error);
            fallback
        }
        Err(message) => {
            throw(env, exception_class::NATIVE_PANIC, &format!("native panic: {message}"));
            fallback
        }
    }
}

fn report(env: &mut JNIEnv<'_>, error: BridgeError) {
    match error {
        BridgeError::Rpx(error) => {
            log::warn!("{error}");
            throw(env, exception_class_for(error.kind()), &error.to_string());
        }
        // A Java exception is already pending (e.g. OutOfMemoryError); let it propagate.
        BridgeError::Jni(jni::errors::Error::JavaException) => {}
        BridgeError::Jni(error) => {
            log::error!("jni error: {error}");
            throw(env, exception_class::INTERNAL, &format!("jni error: {error}"));
        }
    }
}

fn throw(env: &mut JNIEnv<'_>, class: &str, message: &str) {
    // Never overwrite an exception that is already pending.
    if env.exception_check().unwrap_or(false) {
        return;
    }
    if let Err(error) = env.throw_new(class, message) {
        log::error!("could not throw {class}: {error}");
    }
}

fn read_string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> BridgeResult<String> {
    let text: String = env.get_string(value)?.into();
    Ok(text)
}

fn to_handle(raw: jlong) -> BridgeResult<Handle> {
    Handle::from_raw(raw as u64).ok_or_else(|| BridgeError::Rpx(RpxError::InvalidHandle(raw as u64)))
}

/// `static native fun nativeInit(workDir: String)`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeInit<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    work_dir: JString<'local>,
) {
    run(&mut env, (), |env| {
        let dir = read_string(env, &work_dir)?;
        init_engine(&dir)?;
        Ok(())
    })
}

/// `static native fun nativeVersion(): String`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeVersion<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    run(&mut env, std::ptr::null_mut(), |env| {
        Ok(env.new_string(version_string())?.into_raw())
    })
}

/// `static native fun nativeCreateDocument(name: String, mode: Int, width: Int, height: Int): Long`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeCreateDocument<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    name: JString<'local>,
    mode: jint,
    width: jint,
    height: jint,
) -> jlong {
    run(&mut env, 0, |env| {
        let name = read_string(env, &name)?;
        let mode = DocumentMode::from_wire(mode)?;
        let canvas = CanvasSize::from_signed(width, height)?;
        let handle = engine()?.create_document(&name, mode, canvas)?;
        Ok(handle.raw() as jlong)
    })
}

/// `static native fun nativeDocumentInfo(handle: Long): LongArray` (layout: `DocumentInfo::to_wire`)
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeDocumentInfo<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jlongArray {
    run(&mut env, std::ptr::null_mut(), |env| {
        let document = engine()?.document(to_handle(handle)?)?;
        let wire = document.info().to_wire();
        let array = env.new_long_array(wire.len() as jint)?;
        env.set_long_array_region(&array, 0, &wire)?;
        Ok(array.into_raw())
    })
}

/// `static native fun nativeDocumentName(handle: Long): String`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeDocumentName<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jstring {
    run(&mut env, std::ptr::null_mut(), |env| {
        let document = engine()?.document(to_handle(handle)?)?;
        Ok(env.new_string(document.name())?.into_raw())
    })
}

/// `static native fun nativeCloseDocument(handle: Long)`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeCloseDocument<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    run(&mut env, (), |_env| {
        engine()?.close_document(to_handle(handle)?)?;
        Ok(())
    })
}

/// `static native fun nativeOpenDocuments(): LongArray`
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeOpenDocuments<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jlongArray {
    run(&mut env, std::ptr::null_mut(), |env| {
        let raws: Vec<jlong> = engine()?
            .open_documents()
            .into_iter()
            .map(|handle| handle.raw() as jlong)
            .collect();
        let array = env.new_long_array(raws.len() as jint)?;
        env.set_long_array_region(&array, 0, &raws)?;
        Ok(array.into_raw())
    })
}

/// `static native fun nativeDebugPanic()` — proves that panics become Java exceptions.
/// Debug builds panic on purpose; release builds report `Unsupported`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dzekiwolf_wolfpaintdz_core_RustBridge_nativeDebugPanic<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) {
    run(&mut env, (), |_env| {
        if cfg!(debug_assertions) {
            panic!("intentional panic from nativeDebugPanic (debug builds only)");
        }
        Err(BridgeError::Rpx(RpxError::Unsupported(
            "nativeDebugPanic is available in debug builds only",
        )))
    })
}
