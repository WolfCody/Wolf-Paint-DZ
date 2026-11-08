//! RPX engine: document model, opaque handles and session state.
//!
//! Stage 2.1b: `core-model` now depends on `core-raster` for `RasterLayer`/`TileStore`.
//! Dependency direction: core-ffi → core-model → core-raster
//! `core-raster` remains independent (no deps of its own, `unsafe_code = "forbid"`).
//!
//! This crate is free of `unsafe` and `core-raster` has no third-party dependencies,
//! so `cargo test --workspace` runs on any host without Android.

pub mod document;
pub mod engine;
pub mod error;
pub mod handle;

pub use document::{
    CanvasSize, Document, DocumentInfo, DocumentMode, RasterLayer, TileGrid,
    BYTES_PER_PIXEL, MAX_CANVAS_DIMENSION, MAX_CANVAS_PIXELS, MAX_NAME_CHARS, TILE_SIZE,
};
pub use engine::{Engine, EngineConfig};
pub use error::{ErrorKind, Result, RpxError};
pub use handle::{Handle, Registry};

/// Crate version, reported to the UI through `nativeVersion`.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
