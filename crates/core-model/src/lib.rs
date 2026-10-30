//! RPX engine: document model, opaque handles and session state.
//!
//! This crate is deliberately free of `unsafe` and of third-party dependencies, so the
//! whole model can be tested natively with plain `cargo test` (no Android needed).
//! The JNI layer lives in `core-ffi` and only converts types.

pub mod document;
pub mod engine;
pub mod error;
pub mod handle;

pub use document::{
    CanvasSize, Document, DocumentInfo, DocumentMode, TileGrid, BYTES_PER_PIXEL,
    MAX_CANVAS_DIMENSION, MAX_CANVAS_PIXELS, MAX_NAME_CHARS, TILE_SIZE,
};
pub use engine::{Engine, EngineConfig};
pub use error::{ErrorKind, Result, RpxError};
pub use handle::{Handle, Registry};

/// Crate version, reported to the UI through `nativeVersion`.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
