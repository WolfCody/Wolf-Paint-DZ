use core_model::raster_layer::LayerId;
use core_raster::TileStore;
use crate::viewport::Viewport;

pub fn composite(
    layers: &[(LayerId, &TileStore, bool, f32)],
    cw: u32, ch: u32, vp: &Viewport, out: &mut [u8],
) {
    let (sw, sh) = (vp.screen_w as usize, vp.screen_h as usize);
    assert_eq!(out.len(), sw * sh * 4);
    out.fill(0);
    for sy in 0..sh {
        for sx in 0..sw {
            let (cx, cy) = vp.screen_to_canvas(sx as f32 + 0.5, sy as f32 + 0.5);
            let px = cx.floor() as i64; let py = cy.floor() as i64;
            if px < 0 || py < 0 || px >= cw as i64 || py >= ch as i64 { continue; }
            let (px, py) = (px as u32, py as u32);
            let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
            for &(_, store, visible, opacity) in layers {
                if !visible { continue; }
                let p = store.pixel(px, py);
                let pa = p[3] as f32 / 255.0 * opacity;
                if pa < 1e-6 { continue; }
                let inv = 1.0 - pa;
                r = p[0] as f32 / 255.0 * pa + r * inv;
                g = p[1] as f32 / 255.0 * pa + g * inv;
                b = p[2] as f32 / 255.0 * pa + b * inv;
                a = pa + a * inv;
            }
            let i = (sy * sw + sx) * 4;
            if a > 1e-6 {
                out[i]   = (r / a * 255.0).round() as u8;
                out[i+1] = (g / a * 255.0).round() as u8;
                out[i+2] = (b / a * 255.0).round() as u8;
                out[i+3] = (a * 255.0).round() as u8;
            }
        }
    }
}
