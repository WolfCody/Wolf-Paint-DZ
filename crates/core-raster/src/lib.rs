//! Raster tiles for Wolf Paint DZ: the pixel storage that layers will be built on.
//!
//! Stage 2.1a scope (nothing here is wired into the documents or the UI yet):
//! * [`Tile`]: 256 x 256 pixels, RGBA8, premultiplied alpha, stored on the heap;
//! * [`TileStore`]: sparse map of tiles. A tile exists only after something visible was drawn into it,
//!   tiles are shared with snapshots through `Arc` and copied on the first write (copy-on-write), which is
//!   what the undo history will rely on;
//! * geometry helpers: which tile holds a pixel, clipping of rectangles to the canvas, tiles covering a rectangle.
//!
//! The crate is free of `unsafe` and of third-party dependencies, so `cargo test` runs on any host.
//! The pixel format is a decision of `docs/STAGE2_PLAN.md` (ADR 1): RGBA8, premultiplied alpha, sRGB-encoded values.

pub mod geometry;
pub mod tile;
pub mod tile_store;

pub use geometry::{clip_to_canvas, ClippedRect, PixelRect, TileCoord, TileRange};
pub use tile::{premultiply, Rgba, Tile, BYTES_PER_PIXEL, TILE_BYTES, TILE_PIXELS, TILE_SIZE, TRANSPARENT};
pub use tile_store::TileStore;
