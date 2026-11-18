#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushKind { Round, Eraser }

#[derive(Debug, Clone)]
pub struct Brush {
    pub kind: BrushKind,
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub flow: f32,
    pub spacing: f32,
    pub color: [u8; 4],
}

impl Default for Brush {
    fn default() -> Self {
        Brush { kind: BrushKind::Round, size: 20.0, hardness: 0.9,
                opacity: 1.0, flow: 1.0, spacing: 0.25, color: [0, 0, 0, 255] }
    }
}
