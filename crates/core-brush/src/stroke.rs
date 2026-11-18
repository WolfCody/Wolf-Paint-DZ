use crate::brush::Brush;
use crate::dab::paint_dab;
use core_raster::tile_store::TileStore;

pub struct StrokeState { last_x: f32, last_y: f32, distance_acc: f32 }

impl StrokeState {
    pub fn begin(x: f32, y: f32, pressure: f32, store: &mut TileStore, cw: u32, ch: u32, brush: &Brush) -> Self {
        paint_dab(store, cw, ch, brush, x, y, pressure);
        StrokeState { last_x: x, last_y: y, distance_acc: 0.0 }
    }

    pub fn move_to(&mut self, x: f32, y: f32, pressure: f32, store: &mut TileStore, cw: u32, ch: u32, brush: &Brush) {
        let dx = x - self.last_x; let dy = y - self.last_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (brush.size * brush.spacing).max(1.0);
        self.distance_acc += dist;
        while self.distance_acc >= step {
            self.distance_acc -= step;
            let t = 1.0 - self.distance_acc / dist.max(1e-6);
            paint_dab(store, cw, ch, brush, self.last_x + dx * t, self.last_y + dy * t, pressure);
        }
        self.last_x = x; self.last_y = y;
    }

    pub fn end(self, _store: &mut TileStore) {}
}
