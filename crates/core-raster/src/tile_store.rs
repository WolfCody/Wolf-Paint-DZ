//! Sparse tile storage with copy-on-write snapshots and memory accounting.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::geometry::TileCoord;
use crate::tile::{Rgba, Tile, TILE_BYTES, TRANSPARENT};

/// The pixels of one raster layer.
///
/// * **Sparse:** a tile exists only after a visible pixel was written into it. Reading a missing tile gives
///   transparent pixels and allocates nothing; writing a fully transparent pixel into a missing tile allocates nothing.
/// * **Copy-on-write:** [`snapshot`](TileStore::snapshot) copies the map of `Arc`s (pointers, not pixels). A tile that is
///   still shared with a snapshot is copied by the first write that touches it, and only that tile. The undo history
///   keeps snapshots of the tiles a stroke changes.
/// * **Accounting:** [`resident_bytes`](TileStore::resident_bytes) is the memory of the tiles this store references.
///   Tiles shared with another store are counted by each of them; a global budget will deduplicate later.
#[derive(Clone, Default)]
pub struct TileStore {
    tiles: HashMap<TileCoord, Arc<Tile>>,
}

impl TileStore {
    pub fn new() -> TileStore {
        TileStore::default()
    }

    /// Number of allocated tiles.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Memory of the allocated tiles: `len() * 262,144` bytes.
    pub fn resident_bytes(&self) -> usize {
        self.tiles.len() * TILE_BYTES
    }

    pub fn contains(&self, coord: TileCoord) -> bool {
        self.tiles.contains_key(&coord)
    }

    pub fn tile(&self, coord: TileCoord) -> Option<&Tile> {
        self.tiles.get(&coord).map(|tile| tile.as_ref())
    }

    /// The shared handle of a tile, for example to put it into a history entry.
    pub fn shared_tile(&self, coord: TileCoord) -> Option<&Arc<Tile>> {
        self.tiles.get(&coord)
    }

    /// Mutable access to a tile. A missing tile is allocated (transparent); a tile that is shared with a snapshot
    /// is copied first, so the snapshot never changes.
    pub fn tile_mut(&mut self, coord: TileCoord) -> &mut Tile {
        Arc::make_mut(self.tiles.entry(coord).or_insert_with(|| Arc::new(Tile::transparent())))
    }

    /// Stores a tile (for example when undo restores one); returns the tile it replaced.
    pub fn insert(&mut self, coord: TileCoord, tile: Arc<Tile>) -> Option<Arc<Tile>> {
        self.tiles.insert(coord, tile)
    }

    pub fn remove(&mut self, coord: TileCoord) -> Option<Arc<Tile>> {
        self.tiles.remove(&coord)
    }

    /// Frees the tile if it is fully transparent (for example after erasing); returns whether it was freed.
    pub fn prune_transparent(&mut self, coord: TileCoord) -> bool {
        let freeable = self.tiles.get(&coord).is_some_and(|tile| tile.is_transparent());
        if freeable {
            self.tiles.remove(&coord);
        }
        freeable
    }

    /// Frees every fully transparent tile; returns how many were freed.
    pub fn prune_all_transparent(&mut self) -> usize {
        let before = self.tiles.len();
        self.tiles.retain(|_, tile| !tile.is_transparent());
        before - self.tiles.len()
    }

    /// A copy-on-write snapshot: copies the pointers only. Later writes to either store leave the other unchanged.
    pub fn snapshot(&self) -> TileStore {
        self.clone()
    }

    /// Number of tiles that are currently shared with another owner (a snapshot or a history entry).
    pub fn shared_tiles(&self) -> usize {
        self.tiles.values().filter(|tile| Arc::strong_count(tile) > 1).count()
    }

    /// Coordinates in row-major order (top to bottom, left to right): a deterministic order for saving.
    pub fn sorted_coords(&self) -> Vec<TileCoord> {
        let mut coords: Vec<TileCoord> = self.tiles.keys().copied().collect();
        coords.sort_by_key(|coord| (coord.y, coord.x));
        coords
    }

    /// Color of a canvas pixel; transparent where there is no tile. Never allocates.
    pub fn pixel(&self, px: u32, py: u32) -> Rgba {
        let (coord, local_x, local_y) = TileCoord::locate(px, py);
        match self.tiles.get(&coord) {
            Some(tile) => tile.get(local_x, local_y).unwrap_or(TRANSPARENT),
            None => TRANSPARENT,
        }
    }

    /// Writes a canvas pixel. Writing a transparent pixel (alpha 0) into a missing tile does nothing.
    /// The store does not prune after writing a transparent pixel into an existing tile: call
    /// [`prune_transparent`](TileStore::prune_transparent) when a stroke or an erase operation ends.
    pub fn set_pixel(&mut self, px: u32, py: u32, color: Rgba) {
        let (coord, local_x, local_y) = TileCoord::locate(px, py);
        if color[3] == 0 && !self.tiles.contains_key(&coord) {
            return;
        }
        // local coordinates are always < 256, so the write cannot be rejected
        self.tile_mut(coord).set(local_x, local_y, color);
    }
}

impl fmt::Debug for TileStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TileStore {{ tiles: {}, resident_bytes: {} }}", self.len(), self.resident_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::TILE_SIZE;

    const RED: Rgba = [255, 0, 0, 255];
    const BLUE: Rgba = [0, 0, 255, 255];

    #[test]
    fn an_empty_store_holds_nothing_and_reads_transparent() {
        let store = TileStore::new();
        assert_eq!((store.len(), store.resident_bytes()), (0, 0));
        assert!(store.is_empty());
        assert_eq!(store.pixel(0, 0), TRANSPARENT);
        assert_eq!(store.pixel(7999, 7999), TRANSPARENT);
        assert!(store.tile(TileCoord::new(0, 0)).is_none());
    }

    #[test]
    fn reading_never_allocates() {
        let store = TileStore::new();
        for y in (0..8000).step_by(97) {
            for x in (0..8000).step_by(101) {
                assert_eq!(store.pixel(x, y), TRANSPARENT);
            }
        }
        assert!(store.is_empty());
    }

    #[test]
    fn writing_a_transparent_pixel_into_a_missing_tile_allocates_nothing() {
        let mut store = TileStore::new();
        store.set_pixel(10, 10, TRANSPARENT);
        store.set_pixel(5000, 5000, [200, 100, 50, 0]); // alpha 0: counts as transparent
        assert!(store.is_empty());
    }

    #[test]
    fn the_first_visible_pixel_allocates_exactly_one_tile() {
        let mut store = TileStore::new();
        store.set_pixel(300, 10, RED);
        assert_eq!(store.len(), 1);
        assert_eq!(store.resident_bytes(), 262_144);
        assert!(store.contains(TileCoord::new(1, 0)));
        assert_eq!(store.pixel(300, 10), RED);
        assert_eq!(store.pixel(301, 10), TRANSPARENT); // same tile, neighbouring pixel
        assert_eq!(store.pixel(10, 10), TRANSPARENT); // another tile, not allocated by the read
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn many_pixels_in_one_tile_stay_in_one_tile() {
        let mut store = TileStore::new();
        for i in 0..TILE_SIZE {
            store.set_pixel(i, i, RED);
        }
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn pixels_on_both_sides_of_a_tile_edge_use_two_tiles() {
        let mut store = TileStore::new();
        store.set_pixel(255, 0, RED);
        store.set_pixel(256, 0, BLUE);
        assert_eq!(store.len(), 2);
        assert_eq!((store.pixel(255, 0), store.pixel(256, 0)), (RED, BLUE));
        assert_eq!(store.tile(TileCoord::new(0, 0)).unwrap().get(255, 0), Some(RED));
        assert_eq!(store.tile(TileCoord::new(1, 0)).unwrap().get(0, 0), Some(BLUE));
    }

    #[test]
    fn tile_mut_allocates_a_transparent_tile_and_prune_frees_it() {
        let mut store = TileStore::new();
        let coord = TileCoord::new(2, 3);
        assert!(store.tile_mut(coord).is_transparent());
        assert_eq!(store.len(), 1);
        assert!(store.prune_transparent(coord));
        assert!(store.is_empty());
        assert!(!store.prune_transparent(coord), "nothing left to prune");
    }

    #[test]
    fn prune_keeps_tiles_with_visible_pixels() {
        let mut store = TileStore::new();
        store.set_pixel(1, 1, RED);
        assert!(!store.prune_transparent(TileCoord::new(0, 0)));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn erasing_the_only_pixel_leaves_a_tile_that_prune_removes() {
        let mut store = TileStore::new();
        store.set_pixel(1, 1, RED);
        store.set_pixel(1, 1, TRANSPARENT); // existing tile: written, not pruned automatically
        assert_eq!(store.len(), 1);
        assert_eq!(store.prune_all_transparent(), 1);
        assert!(store.is_empty());
    }

    #[test]
    fn prune_all_counts_only_the_transparent_tiles() {
        let mut store = TileStore::new();
        store.set_pixel(1, 1, RED);
        store.tile_mut(TileCoord::new(5, 5));
        store.tile_mut(TileCoord::new(6, 6));
        assert_eq!(store.len(), 3);
        assert_eq!(store.prune_all_transparent(), 2);
        assert_eq!(store.len(), 1);
        assert_eq!(store.pixel(1, 1), RED);
    }

    #[test]
    fn a_snapshot_is_not_changed_by_later_writes_and_only_the_touched_tile_is_copied() {
        let mut store = TileStore::new();
        store.set_pixel(10, 10, RED); // tile (0, 0)
        store.set_pixel(300, 10, BLUE); // tile (1, 0)
        let snapshot = store.snapshot();
        assert_eq!(store.shared_tiles(), 2);

        store.set_pixel(10, 10, [1, 2, 3, 255]);
        assert_eq!(snapshot.pixel(10, 10), RED, "the snapshot must keep the old pixel");
        assert_eq!(store.pixel(10, 10), [1, 2, 3, 255]);

        let untouched_a = store.shared_tile(TileCoord::new(1, 0)).unwrap();
        let untouched_b = snapshot.shared_tile(TileCoord::new(1, 0)).unwrap();
        assert!(Arc::ptr_eq(untouched_a, untouched_b), "an untouched tile stays shared");
        let touched_a = store.shared_tile(TileCoord::new(0, 0)).unwrap();
        let touched_b = snapshot.shared_tile(TileCoord::new(0, 0)).unwrap();
        assert!(!Arc::ptr_eq(touched_a, touched_b), "the touched tile was copied");
        assert_eq!(store.shared_tiles(), 1);
    }

    #[test]
    fn writing_to_an_unshared_tile_does_not_copy_it() {
        let mut store = TileStore::new();
        store.set_pixel(10, 10, RED);
        let before = Arc::as_ptr(store.shared_tile(TileCoord::new(0, 0)).unwrap());
        store.set_pixel(11, 10, BLUE);
        let after = Arc::as_ptr(store.shared_tile(TileCoord::new(0, 0)).unwrap());
        assert_eq!(before, after, "an unshared tile is written in place");
    }

    #[test]
    fn dropping_the_snapshot_makes_the_tiles_private_again() {
        let mut store = TileStore::new();
        store.set_pixel(10, 10, RED);
        let snapshot = store.snapshot();
        assert_eq!(store.shared_tiles(), 1);
        drop(snapshot);
        assert_eq!(store.shared_tiles(), 0);
    }

    #[test]
    fn snapshot_of_the_snapshot_and_independent_histories() {
        let mut store = TileStore::new();
        store.set_pixel(0, 0, RED);
        let first = store.snapshot();
        store.set_pixel(0, 0, BLUE);
        let second = store.snapshot();
        store.set_pixel(0, 0, [9, 9, 9, 255]);
        assert_eq!(first.pixel(0, 0), RED);
        assert_eq!(second.pixel(0, 0), BLUE);
        assert_eq!(store.pixel(0, 0), [9, 9, 9, 255]);
    }

    #[test]
    fn insert_and_remove_restore_a_saved_tile() {
        // what undo does: put the old tile back
        let mut store = TileStore::new();
        store.set_pixel(10, 10, RED);
        let coord = TileCoord::new(0, 0);
        let saved = Arc::clone(store.shared_tile(coord).unwrap());
        store.set_pixel(10, 10, BLUE);
        let replaced = store.insert(coord, saved).unwrap();
        assert_eq!(replaced.get(10, 10), Some(BLUE));
        assert_eq!(store.pixel(10, 10), RED);
        assert!(store.remove(coord).is_some());
        assert!(store.remove(coord).is_none());
    }

    #[test]
    fn sorted_coords_are_row_major_and_deterministic() {
        let mut store = TileStore::new();
        for (x, y) in [(600, 600), (10, 10), (300, 10), (10, 300), (300, 300)] {
            store.set_pixel(x, y, RED);
        }
        let expected = vec![
            TileCoord::new(0, 0),
            TileCoord::new(1, 0),
            TileCoord::new(0, 1),
            TileCoord::new(1, 1),
            TileCoord::new(2, 2),
        ];
        assert_eq!(store.sorted_coords(), expected);
        assert_eq!(store.sorted_coords(), expected);
    }

    #[test]
    fn a_large_canvas_with_100_touched_tiles_holds_about_25_mib_not_256_million_bytes() {
        // the acceptance idea of the plan: an 8000 x 8000 canvas (256,000,000 bytes if dense) with 100 touched tiles
        let dense_bytes: usize = 8000 * 8000 * 4;
        let mut store = TileStore::new();
        for i in 0..10u32 {
            for j in 0..10u32 {
                store.set_pixel(i * 800 + 400, j * 800 + 400, RED);
            }
        }
        assert_eq!(store.len(), 100);
        assert_eq!(store.resident_bytes(), 100 * 262_144);
        assert_eq!(store.resident_bytes(), 26_214_400);
        assert!(store.resident_bytes() < dense_bytes / 9);
    }

    #[test]
    fn debug_output_is_short() {
        let mut store = TileStore::new();
        store.set_pixel(1, 1, RED);
        assert_eq!(format!("{store:?}"), "TileStore { tiles: 1, resident_bytes: 262144 }");
    }

    #[test]
    fn the_store_can_be_moved_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TileStore>();
        let mut store = TileStore::new();
        store.set_pixel(5, 5, RED);
        let snapshot = store.snapshot();
        let handle = std::thread::spawn(move || snapshot.pixel(5, 5));
        store.set_pixel(5, 5, BLUE);
        assert_eq!(handle.join().unwrap(), RED);
        assert_eq!(store.pixel(5, 5), BLUE);
    }
}
