#[derive(Debug, Clone)]
pub struct Viewport {
    pub pan_x: f32, pub pan_y: f32,
    pub zoom: f32, pub rotation_deg: f32,
    pub screen_w: u32, pub screen_h: u32,
}

impl Viewport {
    pub fn new(sw: u32, sh: u32) -> Self {
        Viewport { pan_x: 0.0, pan_y: 0.0, zoom: 1.0, rotation_deg: 0.0, screen_w: sw, screen_h: sh }
    }

    pub fn canvas_to_screen(&self, cx: f32, cy: f32) -> (f32, f32) {
        let (s, c) = self.rotation_deg.to_radians().sin_cos();
        let rx = cx * c - cy * s;
        let ry = cx * s + cy * c;
        (rx * self.zoom + self.pan_x + self.screen_w as f32 * 0.5,
         ry * self.zoom + self.pan_y + self.screen_h as f32 * 0.5)
    }

    pub fn screen_to_canvas(&self, sx: f32, sy: f32) -> (f32, f32) {
        let dx = (sx - self.screen_w as f32 * 0.5 - self.pan_x) / self.zoom;
        let dy = (sy - self.screen_h as f32 * 0.5 - self.pan_y) / self.zoom;
        let (s, c) = (-self.rotation_deg).to_radians().sin_cos();
        (dx * c - dy * s, dx * s + dy * c)
    }

    pub fn fit_canvas(&mut self, cw: u32, ch: u32) {
        self.zoom = (self.screen_w as f32 / cw as f32).min(self.screen_h as f32 / ch as f32) * 0.9;
        self.pan_x = 0.0; self.pan_y = 0.0; self.rotation_deg = 0.0;
    }

    pub fn apply_pan(&mut self, dx: f32, dy: f32) { self.pan_x += dx; self.pan_y += dy; }

    pub fn apply_zoom(&mut self, factor: f32, pivot_sx: f32, pivot_sy: f32) {
        let (cx, cy) = self.screen_to_canvas(pivot_sx, pivot_sy);
        self.zoom = (self.zoom * factor).clamp(1.0 / 32.0, 64.0);
        let (nsx, nsy) = self.canvas_to_screen(cx, cy);
        self.pan_x += pivot_sx - nsx; self.pan_y += pivot_sy - nsy;
    }

    pub fn apply_rotation(&mut self, d: f32) { self.rotation_deg = (self.rotation_deg + d) % 360.0; }
}
