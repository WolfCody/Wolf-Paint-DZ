//! JNI/FFI layer of the RPX engine (`librpx_core.so`).
//!
//! Responsibilities of this crate, and nothing else:
//!   * own the single process-wide `Engine` (`init_engine` / `engine`),
//!   * set up logging (logcat on Android) and the panic hook,
//!   * convert between JNI types and `core-model` types (`jni_bridge`),
//!   * guarantee that no Rust panic ever unwinds into the JVM.
//!
//! All pure helpers live here so they can be unit-tested on the host without a JVM.

#![deny(unsafe_op_in_unsafe_fn)]

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

use core_model::{Engine, EngineConfig, ErrorKind, Result, RpxError};

pub mod jni_bridge;

/// JVM class names (JNI notation) of the exceptions declared in `RpxNativeException.kt`.
/// If a class is renamed on the Kotlin side, change it here too; the instrumented test
/// `RustBridgeInstrumentedTest` throws every kind through the real bridge.
pub mod exception_class {
    pub const INVALID_ARGUMENT: &str = "com/rpxpaint/app/core/RpxNativeException$InvalidArgument";
    pub const CANVAS_TOO_LARGE: &str = "com/rpxpaint/app/core/RpxNativeException$CanvasTooLarge";
    pub const INVALID_HANDLE: &str = "com/rpxpaint/app/core/RpxNativeException$InvalidHandle";
    pub const NOT_INITIALIZED: &str = "com/rpxpaint/app/core/RpxNativeException$NotInitialized";
    pub const UNSUPPORTED: &str = "com/rpxpaint/app/core/RpxNativeException$Unsupported";
    pub const IO: &str = "com/rpxpaint/app/core/RpxNativeException$Io";
    pub const INTERNAL: &str = "com/rpxpaint/app/core/RpxNativeException$Internal";
    /// Raised only by the FFI layer when a Rust panic was caught.
    pub const NATIVE_PANIC: &str = "com/rpxpaint/app/core/RpxNativeException$NativePanic";
}

pub fn exception_class_for(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::InvalidArgument => exception_class::INVALID_ARGUMENT,
        ErrorKind::CanvasTooLarge => exception_class::CANVAS_TOO_LARGE,
        ErrorKind::InvalidHandle => exception_class::INVALID_HANDLE,
        ErrorKind::NotInitialized => exception_class::NOT_INITIALIZED,
        ErrorKind::Unsupported => exception_class::UNSUPPORTED,
        ErrorKind::Io => exception_class::IO,
        ErrorKind::Internal => exception_class::INTERNAL,
    }
}

static ENGINE: OnceLock<Engine> = OnceLock::new();
static LOGGING: Once = Once::new();
static PANIC_HOOK: Once = Once::new();

/// Idempotent. Safe to call again with the same directory (activity/app re-creation).
pub fn init_engine(work_dir: &str) -> Result<()> {
    init_logging();
    install_panic_hook();

    if let Some(existing) = ENGINE.get() {
        if existing.work_dir() != Path::new(work_dir) {
            log::warn!(
                "engine already initialized with {}, ignoring {work_dir}",
                existing.work_dir().display()
            );
        }
        return Ok(());
    }
    let engine = Engine::new(EngineConfig {
        work_dir: PathBuf::from(work_dir),
    })?;
    // Another thread may have won the race; both engines are equivalent, keep the first.
    let _ = ENGINE.set(engine);
    log::info!("engine initialized, work_dir={work_dir}");
    Ok(())
}

pub fn engine() -> Result<&'static Engine> {
    ENGINE.get().ok_or(RpxError::NotInitialized)
}

pub fn version_string() -> String {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    format!(
        "{} ({} {}, {}-bit, {} build)",
        core_model::ENGINE_VERSION,
        std::env::consts::OS,
        std::env::consts::ARCH,
        usize::BITS,
        profile
    )
}

fn init_logging() {
    LOGGING.call_once(|| {
        #[cfg(target_os = "android")]
        {
            let level = if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            };
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(level)
                    .with_tag("rpx-core"),
            );
        }
    });
}

// Rust's default panic hook writes to stderr, which Android discards for apps.
fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        panic::set_hook(Box::new(|info| {
            log::error!("rust panic: {info}");
        }));
    });
}

/// Runs `f` and turns a panic into `Err(message)`. Unwinding across the JNI boundary would
/// abort the process, so every exported function goes through this.
pub fn catch_panic<T>(f: impl FnOnce() -> T) -> std::result::Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(f)).map_err(|payload| panic_message(payload.as_ref()))
}

pub fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&'static str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_kind_maps_to_a_distinct_nested_exception_class() {
        let kinds = [
            ErrorKind::InvalidArgument,
            ErrorKind::CanvasTooLarge,
            ErrorKind::InvalidHandle,
            ErrorKind::NotInitialized,
            ErrorKind::Unsupported,
            ErrorKind::Io,
            ErrorKind::Internal,
        ];
        let mut seen = std::collections::HashSet::new();
        for kind in kinds {
            let class = exception_class_for(kind);
            assert!(class.starts_with("com/rpxpaint/app/core/RpxNativeException$"), "{class}");
            assert!(seen.insert(class), "duplicate mapping for {kind:?}");
        }
        assert!(seen.insert(exception_class::NATIVE_PANIC));
    }

    #[test]
    fn catch_panic_passes_values_through() {
        assert_eq!(catch_panic(|| 41 + 1), Ok(42));
    }

    #[test]
    fn catch_panic_captures_str_and_string_messages() {
        // Silence the default hook output for this test only is not needed: it just prints.
        let first = catch_panic(|| -> () { panic!("static message") });
        assert_eq!(first, Err("static message".to_owned()));
        let n = 7;
        let second = catch_panic(|| -> () { panic!("formatted {n}") });
        assert_eq!(second, Err("formatted 7".to_owned()));
    }

    #[test]
    fn version_string_mentions_crate_version() {
        assert!(version_string().starts_with(core_model::ENGINE_VERSION));
    }

    #[test]
    fn engine_is_unavailable_before_init_and_init_is_idempotent() {
        // This is the only test that touches the process-wide ENGINE.
        if let Err(e) = engine() {
            assert_eq!(e, RpxError::NotInitialized);
        }
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rpx-ffi-{}-{nanos}", std::process::id()));
        let dir_str = dir.to_str().unwrap();
        init_engine(dir_str).unwrap();
        init_engine(dir_str).unwrap();
        assert!(engine().is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }
}
