//! Document model: canvas geometry, tile grid and the document itself.
//!
//! Stage 1 scope: a document knows its name, mode and canvas. It owns no layers and therefore
//! no pixel storage yet; the raster layer/tile store arrives in stage 2. Geometry helpers
//! (tile grid, theoretical dense size) are real and are what the tile store will use.

use crate::error::{Result, RpxError};

/// Edge length of a raster tile in pixels.
pub const TILE_SIZE: u32 = 256;
/// Hard limit per canvas side.
pub const MAX_CANVAS_DIMENSION: u32 = 32_768;
/// Hard limit for width * height (2^30 px, about 1.07 gigapixel).
pub const MAX_CANVAS_PIXELS: u64 = 1 << 30;
/// Pixel format of raster tiles: RGBA, 8 bits per channel.
pub const BYTES_PER_PIXEL: u64 = 4;
/// Maximum document name length in Unicode scalar values.
pub const MAX_NAME_CHARS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentMode {
    Raster,
    Vector,
}

impl DocumentMode {
    /// Value used on the Kotlin side (`DocumentMode.wireValue`).
    pub fn to_wire(self) -> i32 {
        match self {
            DocumentMode::Raster => 0,
            DocumentMode::Vector => 1,
        }
    }

    pub fn from_wire(value: i32) -> Result<Self> {
        match value {
            0 => Ok(DocumentMode::Raster),
            1 => Ok(DocumentMode::Vector),
            other => Err(RpxError::InvalidArgument(format!("unknown document mode {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileGrid {
    pub tiles_x: u32,
    pub tiles_y: u32,
}

impl TileGrid {
    pub fn total(&self) -> u64 {
        u64::from(self.tiles_x) * u64::from(self.tiles_y)
    }
}

/// A validated canvas size. Cannot be constructed out of limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSize {
    width: u32,
    height: u32,
}

impl CanvasSize {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(RpxError::InvalidArgument(
                "canvas width and height must be positive".into(),
            ));
        }
        let pixels = u64::from(width) * u64::from(height);
        if width > MAX_CANVAS_DIMENSION || height > MAX_CANVAS_DIMENSION || pixels > MAX_CANVAS_PIXELS {
            return Err(RpxError::CanvasTooLarge {
                width,
                height,
                max_dimension: MAX_CANVAS_DIMENSION,
                max_pixels: MAX_CANVAS_PIXELS,
            });
        }
        Ok(CanvasSize { width, height })
    }

    /// Entry point for JNI `jint` values: negative numbers are rejected, not wrapped.
    pub fn from_signed(width: i32, height: i32) -> Result<Self> {
        let width = u32::try_from(width)
            .map_err(|_| RpxError::InvalidArgument(format!("negative canvas width {width}")))?;
        let height = u32::try_from(height)
            .map_err(|_| RpxError::InvalidArgument(format!("negative canvas height {height}")))?;
        Self::new(width, height)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Size of a dense RGBA8 buffer for this canvas. It is only a *reference number*: the
    /// engine never allocates it (that is the point of the tile store).
    pub fn dense_rgba_bytes(&self) -> u64 {
        self.pixel_count() * BYTES_PER_PIXEL
    }

    pub fn tile_grid(&self) -> TileGrid {
        TileGrid {
            tiles_x: self.width.div_ceil(TILE_SIZE),
            tiles_y: self.height.div_ceil(TILE_SIZE),
        }
    }
}

/// Snapshot of document facts sent to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentInfo {
    pub mode: DocumentMode,
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub tiles_total: u64,
    pub tiles_allocated: u64,
    pub dense_rgba_bytes: u64,
}

impl DocumentInfo {
    /// Version of the array layout below. Bump together with `DocumentInfo.kt`.
    pub const WIRE_VERSION: i64 = 1;
    pub const WIRE_LEN: usize = 10;

    /// Layout: [version, mode, width, height, tileSize, tilesX, tilesY, tilesTotal,
    /// tilesAllocated, denseRgbaBytes]. Every value fits in a Java `long`.
    pub fn to_wire(&self) -> [i64; Self::WIRE_LEN] {
        [
            Self::WIRE_VERSION,
            i64::from(self.mode.to_wire()),
            i64::from(self.width),
            i64::from(self.height),
            i64::from(self.tile_size),
            i64::from(self.tiles_x),
            i64::from(self.tiles_y),
            self.tiles_total as i64,
            self.tiles_allocated as i64,
            self.dense_rgba_bytes as i64,
        ]
    }
}

#[derive(Debug)]
pub struct Document {
    name: String,
    mode: DocumentMode,
    canvas: CanvasSize,
}

impl Document {
    pub fn new(name: &str, mode: DocumentMode, canvas: CanvasSize) -> Result<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(RpxError::InvalidArgument("document name must not be empty".into()));
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(RpxError::InvalidArgument(format!(
                "document name is longer than {MAX_NAME_CHARS} characters"
            )));
        }
        if name.chars().any(char::is_control) {
            return Err(RpxError::InvalidArgument(
                "document name must not contain control characters".into(),
            ));
        }
        Ok(Document {
            name: name.to_owned(),
            mode,
            canvas,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mode(&self) -> DocumentMode {
        self.mode
    }

    pub fn canvas(&self) -> CanvasSize {
        self.canvas
    }

    pub fn info(&self) -> DocumentInfo {
        let grid = self.canvas.tile_grid();
        DocumentInfo {
            mode: self.mode,
            width: self.canvas.width(),
            height: self.canvas.height(),
            tile_size: TILE_SIZE,
            tiles_x: grid.tiles_x,
            tiles_y: grid.tiles_y,
            tiles_total: grid.total(),
            // Stage 1: a document has no layers, hence no allocated tiles. Stage 2 replaces
            // this with the real count from the tile store.
            tiles_allocated: 0,
            dense_rgba_bytes: self.canvas.dense_rgba_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(w: u32, h: u32) -> CanvasSize {
        CanvasSize::new(w, h).unwrap()
    }

    #[test]
    fn canvas_8000_square_matches_the_spec_numbers() {
        let c = canvas(8000, 8000);
        assert_eq!(c.dense_rgba_bytes(), 256_000_000);
        let grid = c.tile_grid();
        assert_eq!((grid.tiles_x, grid.tiles_y), (32, 32)); // 8000 / 256 = 31.25 -> 32
        assert_eq!(grid.total(), 1024);
    }

    #[test]
    fn tile_grid_rounds_up_on_edges() {
        assert_eq!(canvas(1, 1).tile_grid().total(), 1);
        assert_eq!(canvas(256, 256).tile_grid().total(), 1);
        let g = canvas(257, 256).tile_grid();
        assert_eq!((g.tiles_x, g.tiles_y), (2, 1));
        let g = canvas(512, 513).tile_grid();
        assert_eq!((g.tiles_x, g.tiles_y), (2, 3));
    }

    #[test]
    fn limits_are_inclusive_at_the_edge() {
        assert!(CanvasSize::new(MAX_CANVAS_DIMENSION, MAX_CANVAS_DIMENSION).is_ok());
        assert!(CanvasSize::new(MAX_CANVAS_DIMENSION, 1).is_ok());
    }

    #[test]
    fn rejects_zero_and_oversized_canvas() {
        assert!(matches!(CanvasSize::new(0, 10), Err(RpxError::InvalidArgument(_))));
        assert!(matches!(CanvasSize::new(10, 0), Err(RpxError::InvalidArgument(_))));
        assert!(matches!(
            CanvasSize::new(MAX_CANVAS_DIMENSION + 1, 1),
            Err(RpxError::CanvasTooLarge { .. })
        ));
        assert!(matches!(
            CanvasSize::new(u32::MAX, u32::MAX),
            Err(RpxError::CanvasTooLarge { .. })
        ));
    }

    #[test]
    fn product_of_two_legal_sides_never_overflows_u64() {
        // 32768 * 32767 is the largest product just below the 2^30 pixel budget.
        let c = CanvasSize::new(MAX_CANVAS_DIMENSION, MAX_CANVAS_DIMENSION - 1).unwrap();
        assert_eq!(c.pixel_count(), 32_768 * 32_767);
        assert!(c.pixel_count() < MAX_CANVAS_PIXELS);
        // One pixel row more is rejected by the per-side limit.
        assert!(matches!(
            CanvasSize::new(MAX_CANVAS_DIMENSION, MAX_CANVAS_DIMENSION + 1),
            Err(RpxError::CanvasTooLarge { .. })
        ));
    }

    #[test]
    fn from_signed_rejects_negative_values() {
        assert!(matches!(CanvasSize::from_signed(-1, 10), Err(RpxError::InvalidArgument(_))));
        assert!(matches!(CanvasSize::from_signed(10, i32::MIN), Err(RpxError::InvalidArgument(_))));
        assert_eq!(CanvasSize::from_signed(10, 20).unwrap().width(), 10);
    }

    #[test]
    fn mode_wire_roundtrip_and_unknown_value() {
        for mode in [DocumentMode::Raster, DocumentMode::Vector] {
            assert_eq!(DocumentMode::from_wire(mode.to_wire()).unwrap(), mode);
        }
        assert!(matches!(DocumentMode::from_wire(2), Err(RpxError::InvalidArgument(_))));
        assert!(matches!(DocumentMode::from_wire(-1), Err(RpxError::InvalidArgument(_))));
    }

    #[test]
    fn name_is_trimmed_and_validated() {
        let doc = Document::new("  Sketch  ", DocumentMode::Raster, canvas(10, 10)).unwrap();
        assert_eq!(doc.name(), "Sketch");
        assert!(Document::new("   ", DocumentMode::Raster, canvas(10, 10)).is_err());
        assert!(Document::new("a\nb", DocumentMode::Raster, canvas(10, 10)).is_err());
        let long = "x".repeat(MAX_NAME_CHARS + 1);
        assert!(Document::new(&long, DocumentMode::Raster, canvas(10, 10)).is_err());
        let exact = "x".repeat(MAX_NAME_CHARS);
        assert!(Document::new(&exact, DocumentMode::Raster, canvas(10, 10)).is_ok());
    }

    #[test]
    fn name_length_counts_characters_not_bytes() {
        let cyrillic = "ж".repeat(MAX_NAME_CHARS); // 2 bytes each in UTF-8
        assert!(Document::new(&cyrillic, DocumentMode::Vector, canvas(10, 10)).is_ok());
    }

    #[test]
    fn wire_layout_is_stable() {
        let doc = Document::new("d", DocumentMode::Vector, canvas(8000, 8000)).unwrap();
        let wire = doc.info().to_wire();
        assert_eq!(wire.len(), DocumentInfo::WIRE_LEN);
        assert_eq!(
            wire,
            [1, 1, 8000, 8000, 256, 32, 32, 1024, 0, 256_000_000]
        );
    }

    #[test]
    fn a_new_document_allocates_no_tiles() {
        let doc = Document::new("d", DocumentMode::Raster, canvas(32_768, 32_768)).unwrap();
        let info = doc.info();
        assert_eq!(info.tiles_allocated, 0);
        assert_eq!(info.tiles_total, 128 * 128);
        assert_eq!(info.dense_rgba_bytes, 4 * (1u64 << 30));
    }
}
