use crate::brush::{Brush, BrushKind};
use core_raster::tile_store::TileStore;
use core_raster::geometry::{clip_to_canvas, PixelRect};

pub fn paint_dab(store: &mut TileStore, cw: u32, ch: u32, brush: &Brush, cx: f32, cy: f32, pressure: f32) {
    let radius = (brush.size * 0.5 * pressure.max(0.01)).max(0.5);
    let rc = radius.ceil() as i32;
    let rect = PixelRect { x: (cx - radius).floor() as i32 - 1, y: (cy - radius).floor() as i32 - 1,
                           width: rc as u32 * 2 + 2, height: rc as u32 * 2 + 2 };
    let clip = match clip_to_canvas(rect, cw, ch) { Some(c) => c, None => return };
    let alpha_base = (brush.flow * brush.opacity * pressure).clamp(0.0, 1.0);
    let r2 = radius * radius;
    let hard = brush.hardness.clamp(0.0, 1.0);
    let erase = matches!(brush.kind, BrushKind::Eraser);

    for py in clip.y..clip.y + clip.height {
        for px in clip.x..clip.x + clip.width {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist2 = dx * dx + dy * dy;
            if dist2 > r2 { continue; }
            let dist = dist2.sqrt();
            let soft_start = radius * hard;
            let alpha = if dist <= soft_start { 1.0 }
                        else { 1.0 - (dist - soft_start) / (radius - soft_start + 1e-6) };
            let a = (alpha * alpha_base * 255.0).round() as u8;
            if a == 0 { continue; }
            if erase {
                let old = store.pixel(px, py);
                let new_a = old[3].saturating_sub(a);
                let f = if old[3] > 0 { new_a as f32 / old[3] as f32 } else { 0.0 };
                store.set_pixel(px, py, [(old[0] as f32 * f) as u8, (old[1] as f32 * f) as u8,
                                         (old[2] as f32 * f) as u8, new_a]);
            } else {
                let dst = store.pixel(px, py);
                let src = brush.color;
                let sa = src[3] as f32 / 255.0 * a as f32 / 255.0;
                let da = dst[3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                let blend = |s: u8, d: u8| -> u8 {
                    if out_a < 1e-6 { return 0; }
                    ((s as f32/255.0*sa + d as f32/255.0*da*(1.0-sa))/out_a*255.0).round() as u8
                };
                store.set_pixel(px, py, [blend(src[0],dst[0]), blend(src[1],dst[1]),
                                          blend(src[2],dst[2]), (out_a*255.0).round() as u8]);
            }
        }
    }
}
