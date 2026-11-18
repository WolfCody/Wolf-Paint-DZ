#![deny(unsafe_op_in_unsafe_fn)]
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};
use core_model::{Engine, EngineConfig, ErrorKind, Result, RpxError};

pub mod drawing_session;
pub mod session_registry;
pub mod jni_bridge;

pub mod exception_class {
    pub const INVALID_ARGUMENT: &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$InvalidArgument";
    pub const CANVAS_TOO_LARGE: &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$CanvasTooLarge";
    pub const INVALID_HANDLE:   &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$InvalidHandle";
    pub const NOT_INITIALIZED:  &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$NotInitialized";
    pub const UNSUPPORTED:      &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$Unsupported";
    pub const IO:               &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$Io";
    pub const INTERNAL:         &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$Internal";
    pub const NATIVE_PANIC:     &str = "com/dzekiwolf/wolfpaintdz/core/RpxNativeException$NativePanic";
}

pub fn exception_class_for(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::InvalidArgument => exception_class::INVALID_ARGUMENT,
        ErrorKind::CanvasTooLarge  => exception_class::CANVAS_TOO_LARGE,
        ErrorKind::InvalidHandle   => exception_class::INVALID_HANDLE,
        ErrorKind::NotInitialized  => exception_class::NOT_INITIALIZED,
        ErrorKind::Unsupported     => exception_class::UNSUPPORTED,
        ErrorKind::Io              => exception_class::IO,
        ErrorKind::Internal        => exception_class::INTERNAL,
    }
}

static ENGINE: OnceLock<Engine> = OnceLock::new();
static LOGGING: Once = Once::new();
static PANIC_HOOK: Once = Once::new();

pub fn init_engine(work_dir: &str) -> Result<()> {
    init_logging(); install_panic_hook();
    if ENGINE.get().is_some() { return Ok(()); }
    let engine = Engine::new(EngineConfig { work_dir: PathBuf::from(work_dir) })?;
    let _ = ENGINE.set(engine);
    log::info!("engine initialized");
    Ok(())
}

pub fn engine() -> Result<&'static Engine> { ENGINE.get().ok_or(RpxError::NotInitialized) }

pub fn version_string() -> String {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    format!("{} ({} {}, {}-bit, {} build)", core_model::ENGINE_VERSION,
            std::env::consts::OS, std::env::consts::ARCH, usize::BITS, profile)
}

fn init_logging() {
    LOGGING.call_once(|| {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info })
                .with_tag("rpx-core"),
        );
    });
}

fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        panic::set_hook(Box::new(|info| log::error!("rust panic: {info}")));
    });
}

pub fn catch_panic<T>(f: impl FnOnce() -> T) -> std::result::Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(f)).map_err(|payload| {
        payload.downcast_ref::<&str>().map(|s| (*s).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic".into())
    })
}
