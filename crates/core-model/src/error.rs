//! Error types for the RPX engine. No unsafe, no third-party deps.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpxError {
    InvalidArgument(String),
    CanvasTooLarge { width: u32, height: u32, max_dimension: u32, max_pixels: u64 },
    InvalidHandle(u64),
    NotInitialized,
    Unsupported(&'static str),
    Io(String),
    Internal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidArgument,
    CanvasTooLarge,
    InvalidHandle,
    NotInitialized,
    Unsupported,
    Io,
    Internal,
}

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
            RpxError::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            RpxError::CanvasTooLarge { width, height, max_dimension, max_pixels } => write!(
                f,
                "canvas {width}x{height} exceeds limits (max side {max_dimension}, max pixels {max_pixels})"
            ),
            RpxError::InvalidHandle(raw) => write!(f, "invalid handle: {raw}"),
            RpxError::NotInitialized => write!(f, "engine not initialized"),
            RpxError::Unsupported(m) => write!(f, "unsupported: {m}"),
            RpxError::Io(m) => write!(f, "io error: {m}"),
            RpxError::Internal(m) => write!(f, "internal error: {m}"),
        }
    }
}

impl std::error::Error for RpxError {}

pub type Result<T> = std::result::Result<T, RpxError>;
