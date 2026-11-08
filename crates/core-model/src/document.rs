//! Document model: canvas geometry, tile grid, raster layers and document info.
//!
//! Stage 2.1b: `Document` now owns a `Vec<RasterLayer>`, each wrapping a `TileStore`
//! from `core-raster`. `DocumentInfo::tiles_allocated` reflects the real tile count.

use core_raster::{premultiply, TileStore};

use crate::error::{Result, RpxError};

/// Edge length of a raster tile in pixels.
/// Must equal `core_raster::TILE_SIZE`; `scripts/check_project_consistency.py` enforces it.
pub const TILE_SIZE: u32 = 256;
/// Hard limit per canvas side.
pub const MAX_CANVAS_DIMENSION: u32 = 32_768;
/// Hard limit for width * height (2^30 px, about 1.07 gigapixel).
pub const MAX_CANVAS_PIXELS: u64 = 1 << 30;
/// Pixel format of raster tiles: RGBA, 8 bits per channel.
pub const BYTES_PER_PIXEL: u64 = 4;
/// Maximum document name length in Unicode scalar values.
pub const MAX_NAME_CHARS: usize = 128;

// ── DocumentMode ─────────────────────────────────────────────────────────────

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

// ── TileGrid / CanvasSize ─────────────────────────────────────────────────────

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

    pub fn from_signed(width: i32, height: i32) -> Result<Self> {
        let width = u32::try_from(width)
            .map_err(|_| RpxError::InvalidArgument(format!("negative canvas width {width}")))?;
        let height = u32::try_from(height)
            .map_err(|_| RpxError::InvalidArgument(format!("negative canvas height {height}")))?;
        Self::new(width, height)
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

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

// ── RasterLayer ───────────────────────────────────────────────────────────────

/// One raster layer. Owns its `TileStore` exclusively — no sharing with other layers.
///
/// Invariants:
/// - `opacity` is in `[0.0, 1.0]`.
/// - `id` is unique within its `Document` and never reused.
#[derive(Debug)]
pub struct RasterLayer {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    tiles: TileStore,
}

impl RasterLayer {
    fn new(id: u64, name: String) -> Self {
        RasterLayer { id, name, visible: true, opacity: 1.0, tiles: TileStore::new() }
    }

    /// Number of tiles with at least one written pixel. O(1).
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn tiles(&self) -> &TileStore {
        &self.tiles
    }
}

// ── DocumentInfo ──────────────────────────────────────────────────────────────

/// Snapshot of document facts sent to the UI via `nativeDocumentInfo → LongArray`.
///
/// Wire layout (all slots `i64`):
/// ```text
/// [0]  WIRE_VERSION  = 1
/// [1]  mode          (0=Raster, 1=Vector)
/// [2]  width
/// [3]  height
/// [4]  tileSize      = 256
/// [5]  tilesX
/// [6]  tilesY
/// [7]  tilesTotal
/// [8]  tilesAllocated   ← sum of TileStore::len() across all RasterLayers
/// [9]  denseRgbaBytes
/// ```
/// Never remove or reorder existing slots; bump WIRE_VERSION together with `DocumentInfo.kt`.
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
    pub const WIRE_VERSION: i64 = 1;
    pub const WIRE_LEN: usize = 10;

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

// ── Document ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Document {
    name: String,
    mode: DocumentMode,
    canvas: CanvasSize,
    layers: Vec<RasterLayer>,
    next_layer_id: u64,
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
            layers: Vec::new(),
            next_layer_id: 1,
        })
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn mode(&self) -> DocumentMode { self.mode }
    pub fn canvas(&self) -> CanvasSize { self.canvas }

    // ── Layer management ──────────────────────────────────────────────────────

    /// Adds a new visible, fully-opaque layer. Returns its unique ID.
    pub fn add_layer(&mut self, name: String) -> u64 {
        let id = self.next_layer_id;
        self.next_layer_id += 1;
        self.layers.push(RasterLayer::new(id, name));
        id
    }

    pub fn layer(&self, id: u64) -> Option<&RasterLayer> {
        self.layers.iter().find(|l| l.id == id)
    }

    fn layer_mut(&mut self, id: u64) -> Option<&mut RasterLayer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    // ── Pixel operations ──────────────────────────────────────────────────────

    /// Writes a pixel into the given layer.
    ///
    /// `argb` is the standard Android/Kotlin `0xAARRGGBB` integer.
    /// Premultiplied RGBA conversion happens exactly once here.
    ///
    /// Returns `true` on success.
    /// Returns `false` (no allocation) when the layer does not exist or the
    /// coordinates are outside the canvas.
    pub fn set_pixel(&mut self, layer_id: u64, px: u32, py: u32, argb: u32) -> bool {
        if px >= self.canvas.width() || py >= self.canvas.height() {
            return false;
        }
        let layer = match self.layer_mut(layer_id) {
            Some(l) => l,
            None => return false,
        };
        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;
        let rgba_premul = premultiply([r, g, b, a]);
        layer.tiles.set_pixel(px, py, rgba_premul);
        true
    }

    // ── Accounting ────────────────────────────────────────────────────────────

    /// Sum of `TileStore::len()` across all layers (visible and hidden). O(layers).
    pub fn tiles_allocated(&self) -> usize {
        self.layers.iter().map(|l| l.tile_count()).sum()
    }

    // ── Info ──────────────────────────────────────────────────────────────────

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
            tiles_allocated: self.tiles_allocated() as u64,
            dense_rgba_bytes: self.canvas.dense_rgba_bytes(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(w: u32, h: u32) -> CanvasSize { CanvasSize::new(w, h).unwrap() }
    fn doc() -> Document { Document::new("test", DocumentMode::Raster, canvas(800, 600)).unwrap() }

    // ── Existing Stage 1 tests (unchanged) ────────────────────────────────────

    #[test]
    fn canvas_8000_square_matches_the_spec_numbers() {
        let c = canvas(8000, 8000);
        assert_eq!(c.dense_rgba_bytes(), 256_000_000);
        let grid = c.tile_grid();
        assert_eq!((grid.tiles_x, grid.tiles_y), (32, 32));
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
        let c = CanvasSize::new(MAX_CANVAS_DIMENSION, MAX_CANVAS_DIMENSION - 1).unwrap();
        assert_eq!(c.pixel_count(), 32_768 * 32_767);
        assert!(c.pixel_count() < MAX_CANVAS_PIXELS);
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
        let cyrillic = "ж".repeat(MAX_NAME_CHARS);
        assert!(Document::new(&cyrillic, DocumentMode::Vector, canvas(10, 10)).is_ok());
    }

    #[test]
    fn wire_layout_is_stable() {
        let doc = Document::new("d", DocumentMode::Vector, canvas(8000, 8000)).unwrap();
        let wire = doc.info().to_wire();
        assert_eq!(wire.len(), DocumentInfo::WIRE_LEN);
        // tilesAllocated is 0 for a new document with no layers.
        assert_eq!(wire, [1, 1, 8000, 8000, 256, 32, 32, 1024, 0, 256_000_000]);
    }

    #[test]
    fn a_new_document_allocates_no_tiles() {
        let doc = Document::new("d", DocumentMode::Raster, canvas(32_768, 32_768)).unwrap();
        let info = doc.info();
        assert_eq!(info.tiles_allocated, 0);
        assert_eq!(info.tiles_total, 128 * 128);
        assert_eq!(info.dense_rgba_bytes, 4 * (1u64 << 30));
    }

    // ── New Stage 2.1b: RasterLayer ───────────────────────────────────────────

    #[test]
    fn raster_layer_defaults_are_visible_opaque_and_empty() {
        let layer = RasterLayer::new(1, "Background".into());
        assert_eq!(layer.id, 1);
        assert_eq!(layer.name, "Background");
        assert!(layer.visible, "new layer must be visible");
        assert!((layer.opacity - 1.0).abs() < f32::EPSILON, "new layer must be fully opaque");
        assert_eq!(layer.tile_count(), 0);
    }

    #[test]
    fn add_layer_returns_unique_increasing_ids() {
        let mut d = doc();
        let id1 = d.add_layer("A".into());
        let id2 = d.add_layer("B".into());
        let id3 = d.add_layer("C".into());
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert!(id2 > id1);
        assert!(id3 > id2);
    }

    #[test]
    fn add_one_layer_accessible_by_id() {
        let mut d = doc();
        let id = d.add_layer("Background".into());
        let layer = d.layer(id).unwrap();
        assert_eq!(layer.name, "Background");
        assert!(layer.visible);
    }

    #[test]
    fn add_multiple_layers_all_accessible() {
        let mut d = doc();
        let id1 = d.add_layer("A".into());
        let id2 = d.add_layer("B".into());
        assert_eq!(d.layer(id1).unwrap().name, "A");
        assert_eq!(d.layer(id2).unwrap().name, "B");
    }

    #[test]
    fn get_layer_unknown_id_returns_none() {
        let mut d = doc();
        d.add_layer("X".into());
        assert!(d.layer(999).is_none());
    }

    #[test]
    fn new_document_has_no_layers() {
        let d = doc();
        assert_eq!(d.tiles_allocated(), 0);
        assert!(d.layer(1).is_none());
    }

    // ── TileStore independence ─────────────────────────────────────────────────

    #[test]
    fn layers_have_independent_tile_stores() {
        let mut d = doc();
        let id1 = d.add_layer("A".into());
        let id2 = d.add_layer("B".into());
        assert!(d.set_pixel(id1, 0, 0, 0xFF_FF0000));
        assert_eq!(d.layer(id1).unwrap().tile_count(), 1);
        assert_eq!(d.layer(id2).unwrap().tile_count(), 0, "layer 2 must be untouched");
    }

    // ── set_pixel ────────────────────────────────────────────────────────────

    #[test]
    fn set_pixel_into_existing_layer_returns_true() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        assert!(d.set_pixel(id, 10, 20, 0xFF_112233));
    }

    #[test]
    fn set_pixel_into_unknown_layer_returns_false() {
        let mut d = doc();
        assert!(!d.set_pixel(999, 0, 0, 0xFF_FF0000));
    }

    #[test]
    fn set_pixel_outside_canvas_returns_false_and_no_tile_allocated() {
        let mut d = Document::new("t", DocumentMode::Raster, canvas(100, 100)).unwrap();
        let id = d.add_layer("L".into());
        assert!(!d.set_pixel(id, 100, 0, 0xFF_FF0000));
        assert!(!d.set_pixel(id, 0, 100, 0xFF_FF0000));
        assert_eq!(d.tiles_allocated(), 0);
    }

    #[test]
    fn pixel_isolation_between_layers() {
        let mut d = doc();
        let id1 = d.add_layer("A".into());
        let id2 = d.add_layer("B".into());
        d.set_pixel(id1, 5, 5, 0xFF_FF0000);
        d.set_pixel(id2, 5, 5, 0xFF_0000FF);
        // Both layers touched the same canvas position but have separate tile stores.
        assert_eq!(d.layer(id1).unwrap().tile_count(), 1);
        assert_eq!(d.layer(id2).unwrap().tile_count(), 1);
        // Read back pixels independently.
        use core_raster::TileCoord;
        let (coord, lx, ly) = TileCoord::locate(5, 5);
        let p1 = d.layer(id1).unwrap().tiles().tile(coord).unwrap().get(lx, ly).unwrap();
        let p2 = d.layer(id2).unwrap().tiles().tile(coord).unwrap().get(lx, ly).unwrap();
        assert_ne!(p1, p2, "pixels in different layers must be independent");
    }

    #[test]
    fn repeated_write_does_not_allocate_extra_tile() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 0, 0, 0xFF_FF0000);
        d.set_pixel(id, 0, 0, 0xFF_00FF00);
        assert_eq!(d.tiles_allocated(), 1);
    }

    #[test]
    fn argb_to_premultiplied_rgba_conversion() {
        // 0x80_FF_00_00: a=128, r=255, g=0, b=0 → premultiplied r ≈ 128
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 0, 0, 0x80_FF_00_00);
        use core_raster::TileCoord;
        let (coord, lx, ly) = TileCoord::locate(0, 0);
        let pixel = d.layer(id).unwrap().tiles().tile(coord).unwrap().get(lx, ly).unwrap();
        assert_eq!(pixel[3], 128);
        assert!(pixel[0] <= 128);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 0);
    }

    #[test]
    fn fully_transparent_pixel_into_missing_tile_does_not_allocate() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 0, 0, 0x00_00_00_00);
        assert_eq!(d.tiles_allocated(), 0);
    }

    // ── tiles_allocated ────────────────────────────────────────────────────────

    #[test]
    fn tiles_allocated_initial_is_zero() {
        assert_eq!(doc().tiles_allocated(), 0);
    }

    #[test]
    fn tiles_allocated_counts_hidden_layers_too() {
        let mut d = doc();
        let id1 = d.add_layer("Visible".into());
        let id2 = d.add_layer("Hidden".into());
        d.layer_mut(id2).unwrap().visible = false;
        d.set_pixel(id1, 0, 0, 0xFF_FF0000);
        d.set_pixel(id2, 0, 0, 0xFF_0000FF);
        assert_eq!(d.tiles_allocated(), 2, "hidden layers must be counted");
    }

    #[test]
    fn tiles_allocated_sums_across_layers() {
        let mut d = doc();
        let id1 = d.add_layer("A".into());
        let id2 = d.add_layer("B".into());
        d.set_pixel(id1, 0, 0, 0xFF_FF0000);    // tile (0,0)
        d.set_pixel(id2, 300, 0, 0xFF_0000FF);  // tile (1,0) in layer 2
        assert_eq!(d.tiles_allocated(), 2);
    }

    #[test]
    fn tiles_allocated_counts_multiple_tiles_per_layer() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 0, 0, 0xFF_FF0000);    // tile (0,0)
        d.set_pixel(id, 300, 0, 0xFF_00FF00);  // tile (1,0)
        d.set_pixel(id, 0, 300, 0xFF_0000FF);  // tile (0,1)
        assert_eq!(d.tiles_allocated(), 3);
    }

    // ── Wire: tilesAllocated field ─────────────────────────────────────────────

    #[test]
    fn tiles_allocated_field_is_at_slot_eight() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 0, 0, 0xFF_FF0000);
        let wire = d.info().to_wire();
        assert_eq!(wire[8], 1, "tilesAllocated must be at slot 8");
    }

    #[test]
    fn tiles_allocated_initial_value_is_zero_in_wire() {
        let wire = doc().info().to_wire();
        assert_eq!(wire[8], 0);
    }

    #[test]
    fn tiles_allocated_updates_after_pixel_write_in_wire() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 0, 0, 0xFF_FF0000);
        assert_eq!(d.info().to_wire()[8], 1);
    }

    #[test]
    fn previous_wire_fields_preserved() {
        let d = Document::new("d", DocumentMode::Vector, canvas(8000, 8000)).unwrap();
        let wire = d.info().to_wire();
        assert_eq!(wire[0], 1);     // version
        assert_eq!(wire[1], 1);     // Vector
        assert_eq!(wire[2], 8000);  // width
        assert_eq!(wire[3], 8000);  // height
        assert_eq!(wire[4], 256);   // tileSize
        assert_eq!(wire[5], 32);    // tilesX
        assert_eq!(wire[6], 32);    // tilesY
        assert_eq!(wire[7], 1024);  // tilesTotal
        assert_eq!(wire[8], 0);     // tilesAllocated (no layers)
        assert_eq!(wire[9], 256_000_000); // denseRgbaBytes
    }

    // ── TileStore integration ──────────────────────────────────────────────────

    #[test]
    fn multiple_pixels_in_one_tile_stay_in_one_tile() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        for i in 0..10u32 {
            d.set_pixel(id, i, i, 0xFF_FF0000);
        }
        assert_eq!(d.tiles_allocated(), 1);
    }

    #[test]
    fn pixels_across_tile_boundary_use_two_tiles() {
        let mut d = doc();
        let id = d.add_layer("L".into());
        d.set_pixel(id, 255, 0, 0xFF_FF0000);
        d.set_pixel(id, 256, 0, 0xFF_0000FF);
        assert_eq!(d.tiles_allocated(), 2);
    }
}
