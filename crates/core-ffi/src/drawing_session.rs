use std::collections::HashMap;
use std::sync::Arc;
use core_model::raster_layer::LayerId;
use core_raster::{Tile, TileCoord, TileStore};
use core_brush::{Brush, StrokeState};
use core_history::{History, StrokeEntry, TileSnapshot};
use core_render::Viewport;

pub struct DrawingSession {
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub layer_order: Vec<LayerId>,
    pub layer_visible: HashMap<LayerId, bool>,
    pub layer_opacity: HashMap<LayerId, f32>,
    pub stores: HashMap<LayerId, TileStore>,
    pub active_layer: Option<LayerId>,
    pub history: History,
    pub viewport: Viewport,
    pub brush: Brush,
    stroke: Option<(StrokeState, HashMap<TileCoord, Option<Arc<Tile>>>)>,
    stroke_layer: Option<LayerId>,
}

impl DrawingSession {
    pub fn new(cw: u32, ch: u32) -> Self {
        DrawingSession {
            canvas_w: cw, canvas_h: ch,
            layer_order: Vec::new(),
            layer_visible: HashMap::new(),
            layer_opacity: HashMap::new(),
            stores: HashMap::new(),
            active_layer: None,
            history: History::default_budget(),
            viewport: Viewport::new(cw, ch),
            brush: Brush::default(),
            stroke: None,
            stroke_layer: None,
        }
    }

    pub fn add_layer(&mut self, id: LayerId) {
        self.stores.entry(id).or_insert_with(TileStore::new);
        self.layer_visible.insert(id, true);
        self.layer_opacity.insert(id, 1.0);
        if !self.layer_order.contains(&id) { self.layer_order.push(id); }
        if self.active_layer.is_none() { self.active_layer = Some(id); }
    }

    pub fn remove_layer(&mut self, id: LayerId) {
        self.stores.remove(&id);
        self.layer_visible.remove(&id);
        self.layer_opacity.remove(&id);
        self.layer_order.retain(|&l| l != id);
        if self.active_layer == Some(id) { self.active_layer = self.layer_order.last().copied(); }
    }

    pub fn stroke_begin(&mut self, x: f32, y: f32, pressure: f32) {
        let lid = match self.active_layer { Some(l) => l, None => return };
        let store = self.stores.entry(lid).or_insert_with(TileStore::new);
        let snap_before: HashMap<TileCoord, Option<Arc<Tile>>> = HashMap::new();
        let state = StrokeState::begin(x, y, pressure, store, self.canvas_w, self.canvas_h, &self.brush);
        self.stroke = Some((state, snap_before));
        self.stroke_layer = Some(lid);
    }

    pub fn stroke_move(&mut self, x: f32, y: f32, pressure: f32) {
        let lid = match self.stroke_layer { Some(l) => l, None => return };
        let store = self.stores.entry(lid).or_insert_with(TileStore::new);
        if let Some((state, _)) = &mut self.stroke {
            state.move_to(x, y, pressure, store, self.canvas_w, self.canvas_h, &self.brush);
        }
    }

    pub fn stroke_end(&mut self) {
        if let (Some((state, snaps)), Some(lid)) = (self.stroke.take(), self.stroke_layer.take()) {
            if let Some(store) = self.stores.get_mut(&lid) { state.end(store); }
            let tiles = snaps.into_iter().map(|(coord, before)| TileSnapshot { layer: lid, coord, before }).collect();
            self.history.push(StrokeEntry { tiles });
        }
    }

    pub fn stroke_cancel(&mut self) { self.stroke = None; self.stroke_layer = None; }

    pub fn undo(&mut self) -> bool { self.history.undo(&mut self.stores) }
    pub fn redo(&mut self) -> bool { self.history.redo(&mut self.stores) }

    pub fn tiles_allocated(&self) -> usize { self.stores.values().map(|s| s.len()).sum() }

    pub fn render_to_buffer(&self, out: &mut [u8]) {
        let empty = TileStore::new();
        let layers: Vec<_> = self.layer_order.iter().map(|id| {
            let store: &TileStore = self.stores.get(id).unwrap_or(&empty);
            let visible = self.layer_visible.get(id).copied().unwrap_or(true);
            let opacity = self.layer_opacity.get(id).copied().unwrap_or(1.0);
            (*id, store, visible, opacity)
        }).collect();
        core_render::composite(&layers, self.canvas_w, self.canvas_h, &self.viewport, out);
    }
}
