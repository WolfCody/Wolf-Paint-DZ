//! Error types shared by the whole engine.

use std::fmt;

/// Coarse error category. The FFI layer maps each kind to one Kotlin exception class,
/// so adding a kind here means adding the class in `RpxNativeException.kt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    InvalidArgument,
    CanvasTooLarge,
    InvalidHandle,
    NotInitialized,
    Unsupported,
    Io,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpxError {
    /// A caller-supplied value is malformed (empty name, unknown mode, size <= 0, ...).
    InvalidArgument(String),
    /// Canvas exceeds `MAX_CANVAS_DIMENSION` per side or `MAX_CANVAS_PIXELS` in total.
    CanvasTooLarge {
        width: u32,
        height: u32,
        max_dimension: u32,
        max_pixels: u64,
    },
    /// Handle is malformed, already closed, or was never issued. Carries the raw value.
    InvalidHandle(u64),
    /// `nativeInit` has not completed successfully.
    NotInitialized,
    /// Operation is not available in this build or stage.
    Unsupported(&'static str),
    /// File-system problem (message already contains the path).
    Io(String),
    /// Invariant violation inside the engine.
    Internal(String),
}

pub type Result<T> = std::result::Result<T, RpxError>;

impl RpxError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            RpxError::InvalidArgument(_) => ErrorKind::InvalidArgument,
            RpxError::CanvasTooLarge { .. } => ErrorKind::CanvasTooLarge,
            RpxError::InvalidHandle(_) => ErrorKind::InvalidHandle,
            RpxError::NotInitialized => ErrorKind::NotInitialized,
            RpxError::Unsupported(_) => ErrorKind::Unsupported,
            RpxError::Io(_) => ErrorKind::Io,
            RpxError::Internal(_) => ErrorKind::Internal,
        }
    }
}

impl fmt::Display for RpxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpxError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            RpxError::CanvasTooLarge {
                width,
                height,
                max_dimension,
                max_pixels,
            } => write!(
                f,
                "canvas {width}x{height} is too large (max {max_dimension} px per side, {max_pixels} px total)"
            ),
            RpxError::InvalidHandle(raw) => write!(f, "invalid or closed handle {raw}"),
            RpxError::NotInitialized => write!(f, "engine is not initialized"),
            RpxError::Unsupported(what) => write!(f, "unsupported: {what}"),
            RpxError::Io(msg) => write!(f, "i/o error: {msg}"),
            RpxError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for RpxError {}
