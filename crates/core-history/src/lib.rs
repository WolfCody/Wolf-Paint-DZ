use std::collections::HashMap;
use std::sync::Arc;
use core_model::raster_layer::LayerId;
use core_raster::{Tile, TileCoord, TileStore, TILE_BYTES};

pub const DEFAULT_BUDGET: usize = 256 * 1024 * 1024;

pub struct TileSnapshot {
    pub layer: LayerId,
    pub coord: TileCoord,
    pub before: Option<Arc<Tile>>,
}

pub struct StrokeEntry { pub tiles: Vec<TileSnapshot> }

pub struct History {
    past: Vec<StrokeEntry>,
    future: Vec<StrokeEntry>,
    budget: usize,
    used: usize,
}

impl History {
    pub fn new(budget: usize) -> Self { History { past: Vec::new(), future: Vec::new(), budget, used: 0 } }
    pub fn default_budget() -> Self { Self::new(DEFAULT_BUDGET) }

    pub fn push(&mut self, entry: StrokeEntry) {
        let cost = entry.tiles.len() * TILE_BYTES;
        while self.used + cost > self.budget && !self.past.is_empty() {
            if let Some(e) = self.past.first() {
                self.used = self.used.saturating_sub(e.tiles.len() * TILE_BYTES);
            }
            self.past.remove(0);
        }
        self.used += cost;
        self.past.push(entry);
        self.future.clear();
    }

    pub fn can_undo(&self) -> bool { !self.past.is_empty() }
    pub fn can_redo(&self) -> bool { !self.future.is_empty() }

    pub fn undo(&mut self, stores: &mut HashMap<LayerId, TileStore>) -> bool {
        let entry = match self.past.pop() { Some(e) => e, None => return false };
        let redo = swap_tiles(entry, stores);
        self.future.push(redo);
        true
    }

    pub fn redo(&mut self, stores: &mut HashMap<LayerId, TileStore>) -> bool {
        let entry = match self.future.pop() { Some(e) => e, None => return false };
        let undo = swap_tiles(entry, stores);
        self.past.push(undo);
        true
    }
}

fn swap_tiles(entry: StrokeEntry, stores: &mut HashMap<LayerId, TileStore>) -> StrokeEntry {
    let mut inverse = Vec::new();
    for snap in entry.tiles {
        let store = stores.entry(snap.layer).or_insert_with(TileStore::new);
        let current = store.shared_tile(snap.coord).map(Arc::clone);
        match &snap.before {
            Some(t) => { store.insert(snap.coord, Arc::clone(t)); }
            None    => { store.remove(snap.coord); }
        }
        inverse.push(TileSnapshot { layer: snap.layer, coord: snap.coord, before: current });
    }
    StrokeEntry { tiles: inverse }
}
