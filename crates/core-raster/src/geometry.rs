//! Canvas/tile geometry: which tile holds a pixel, clipping to the canvas, tiles covering a rectangle.
//! All arithmetic is checked or done in `i64`, so extreme values from the outside never overflow.

use crate::tile::TILE_SIZE;

/// Index of a tile in the tile grid (column `x`, row `y`). The canvas starts at (0, 0): no negative indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub const fn new(x: u32, y: u32) -> TileCoord {
        TileCoord { x, y }
    }

    /// The tile that holds canvas pixel (`px`, `py`) and the pixel's position inside that tile.
    pub const fn locate(px: u32, py: u32) -> (TileCoord, u32, u32) {
        (
            TileCoord { x: px / TILE_SIZE, y: py / TILE_SIZE },
            px % TILE_SIZE,
            py % TILE_SIZE,
        )
    }

    /// Canvas pixel of the tile's top-left corner; `None` if it does not fit into `u32`.
    pub fn origin(self) -> Option<(u32, u32)> {
        Some((self.x.checked_mul(TILE_SIZE)?, self.y.checked_mul(TILE_SIZE)?))
    }
}

/// A rectangle in canvas pixels. It may reach outside the canvas (negative origin, too large).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A non-empty rectangle that lies completely inside the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClippedRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The part of `rect` that is inside a `canvas_width` x `canvas_height` canvas; `None` if nothing is inside.
pub fn clip_to_canvas(rect: PixelRect, canvas_width: u32, canvas_height: u32) -> Option<ClippedRect> {
    let x0 = i64::from(rect.x).max(0);
    let y0 = i64::from(rect.y).max(0);
    let x1 = (i64::from(rect.x) + i64::from(rect.width)).min(i64::from(canvas_width));
    let y1 = (i64::from(rect.y) + i64::from(rect.height)).min(i64::from(canvas_height));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    // 0 <= x0 < x1 <= canvas_width (u32), so every value below fits into u32
    Some(ClippedRect {
        x: x0 as u32,
        y: y0 as u32,
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
    })
}

/// A block of tiles `[x0, x1) x [y0, y1)` (half-open, never empty when built by the constructors below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRange {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl TileRange {
    /// The tiles that a clipped rectangle touches; `None` for an empty rectangle.
    pub fn covering(rect: ClippedRect) -> Option<TileRange> {
        let last_x = rect.x.checked_add(rect.width.checked_sub(1)?)?;
        let last_y = rect.y.checked_add(rect.height.checked_sub(1)?)?;
        Some(TileRange {
            x0: rect.x / TILE_SIZE,
            y0: rect.y / TILE_SIZE,
            x1: last_x / TILE_SIZE + 1,
            y1: last_y / TILE_SIZE + 1,
        })
    }

    /// All tiles of a `width` x `height` canvas; `None` for an empty canvas.
    pub fn whole_canvas(width: u32, height: u32) -> Option<TileRange> {
        TileRange::covering(ClippedRect { x: 0, y: 0, width, height })
    }

    pub fn width_in_tiles(&self) -> u32 {
        self.x1 - self.x0
    }

    pub fn height_in_tiles(&self) -> u32 {
        self.y1 - self.y0
    }

    pub fn count(&self) -> u64 {
        u64::from(self.width_in_tiles()) * u64::from(self.height_in_tiles())
    }

    pub fn contains(&self, tile: TileCoord) -> bool {
        tile.x >= self.x0 && tile.x < self.x1 && tile.y >= self.y0 && tile.y < self.y1
    }

    /// Every tile of the range, row by row (left to right, top to bottom).
    pub fn coords(self) -> impl Iterator<Item = TileCoord> {
        let (x0, x1, y0, y1) = (self.x0, self.x1, self.y0, self.y1);
        (y0..y1).flat_map(move |y| (x0..x1).map(move |x| TileCoord { x, y }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Small deterministic pseudo-random generator (no dependencies).
    struct Lcg(u64);
    impl Lcg {
        fn next_u32(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (self.0 >> 33) as u32
        }
        fn range(&mut self, lo: i32, hi: i32) -> i32 {
            lo + (self.next_u32() % ((hi - lo) as u32)) as i32
        }
    }

    #[test]
    fn locate_handles_tile_edges() {
        assert_eq!(TileCoord::locate(0, 0), (TileCoord::new(0, 0), 0, 0));
        assert_eq!(TileCoord::locate(255, 255), (TileCoord::new(0, 0), 255, 255));
        assert_eq!(TileCoord::locate(256, 0), (TileCoord::new(1, 0), 0, 0));
        assert_eq!(TileCoord::locate(0, 512), (TileCoord::new(0, 2), 0, 0));
        // the last pixel of an 8000 pixel canvas: 7999 = 31 * 256 + 63
        assert_eq!(TileCoord::locate(7999, 7999), (TileCoord::new(31, 31), 63, 63));
    }

    #[test]
    fn tile_origin_and_overflow() {
        assert_eq!(TileCoord::new(3, 2).origin(), Some((768, 512)));
        assert_eq!(TileCoord::new(0, 0).origin(), Some((0, 0)));
        assert_eq!(TileCoord::new(u32::MAX, 0).origin(), None);
        assert_eq!(TileCoord::new(0, u32::MAX / 2).origin(), None);
    }

    #[test]
    fn locate_and_origin_agree() {
        for (px, py) in [(0, 0), (1, 1), (255, 256), (1000, 3000), (32767, 32767)] {
            let (tile, lx, ly) = TileCoord::locate(px, py);
            let (ox, oy) = tile.origin().unwrap();
            assert_eq!((ox + lx, oy + ly), (px, py));
        }
    }

    #[test]
    fn clipping_keeps_the_inside_part() {
        let canvas = (300, 200);
        let clip = |x, y, w, h| clip_to_canvas(PixelRect { x, y, width: w, height: h }, canvas.0, canvas.1);
        assert_eq!(clip(10, 10, 20, 20), Some(ClippedRect { x: 10, y: 10, width: 20, height: 20 }));
        assert_eq!(clip(-5, -5, 20, 20), Some(ClippedRect { x: 0, y: 0, width: 15, height: 15 }));
        assert_eq!(clip(290, 190, 50, 50), Some(ClippedRect { x: 290, y: 190, width: 10, height: 10 }));
        assert_eq!(clip(-100, -100, 1000, 1000), Some(ClippedRect { x: 0, y: 0, width: 300, height: 200 }));
    }

    #[test]
    fn clipping_rejects_empty_and_outside_rectangles() {
        let clip = |x, y, w, h| clip_to_canvas(PixelRect { x, y, width: w, height: h }, 300, 200);
        assert_eq!(clip(10, 10, 0, 5), None);
        assert_eq!(clip(10, 10, 5, 0), None);
        assert_eq!(clip(300, 0, 10, 10), None); // starts exactly at the right edge
        assert_eq!(clip(0, 200, 10, 10), None);
        assert_eq!(clip(-10, 0, 10, 10), None); // ends exactly at the left edge
        assert_eq!(clip(-500, -500, 100, 100), None);
        assert!(clip(0, 0, 10, 10).is_some());
    }

    #[test]
    fn clipping_survives_extreme_values() {
        // from i32::MIN to i32::MAX: covers the whole canvas, and the sum does not overflow
        let r = PixelRect { x: i32::MIN, y: i32::MIN, width: u32::MAX, height: u32::MAX };
        assert_eq!(clip_to_canvas(r, 8000, 8000), Some(ClippedRect { x: 0, y: 0, width: 8000, height: 8000 }));
        let r = PixelRect { x: i32::MAX, y: i32::MAX, width: u32::MAX, height: u32::MAX };
        assert_eq!(clip_to_canvas(r, 8000, 8000), None);
        let r = PixelRect { x: -1, y: -1, width: u32::MAX, height: u32::MAX };
        assert_eq!(clip_to_canvas(r, 8000, 8000), Some(ClippedRect { x: 0, y: 0, width: 8000, height: 8000 }));
        assert_eq!(clip_to_canvas(PixelRect { x: 0, y: 0, width: 5, height: 5 }, 0, 0), None);
    }

    #[test]
    fn clipping_matches_a_brute_force_on_a_tiny_canvas() {
        let (w, h) = (5u32, 4u32);
        for x in -6..8 {
            for y in -5..7 {
                for rw in 0..8u32 {
                    for rh in 0..8u32 {
                        let mut inside = Vec::new();
                        for px in x..x + rw as i32 {
                            for py in y..y + rh as i32 {
                                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                                    inside.push((px as u32, py as u32));
                                }
                            }
                        }
                        let clipped = clip_to_canvas(PixelRect { x, y, width: rw, height: rh }, w, h);
                        match clipped {
                            None => assert!(inside.is_empty(), "rect {x},{y} {rw}x{rh} lost {inside:?}"),
                            Some(c) => {
                                assert_eq!(c.width as usize * c.height as usize, inside.len(), "rect {x},{y} {rw}x{rh}");
                                for (px, py) in inside {
                                    assert!(px >= c.x && px < c.x + c.width && py >= c.y && py < c.y + c.height);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn covering_counts_tiles_at_the_boundaries() {
        let cover = |x, y, w, h| TileRange::covering(ClippedRect { x, y, width: w, height: h }).unwrap();
        let one = cover(0, 0, 256, 256);
        assert_eq!((one.width_in_tiles(), one.height_in_tiles(), one.count()), (1, 1, 1));
        let two_wide = cover(0, 0, 257, 1);
        assert_eq!((two_wide.width_in_tiles(), two_wide.height_in_tiles()), (2, 1));
        let across = cover(255, 255, 2, 2);
        assert_eq!((across.x0, across.y0, across.x1, across.y1), (0, 0, 2, 2));
        let second_only = cover(256, 0, 256, 1);
        assert_eq!((second_only.x0, second_only.x1), (1, 2));
        assert!(TileRange::covering(ClippedRect { x: 0, y: 0, width: 0, height: 5 }).is_none());
    }

    #[test]
    fn whole_canvas_matches_the_document_numbers() {
        let eight_thousand = TileRange::whole_canvas(8000, 8000).unwrap();
        assert_eq!((eight_thousand.width_in_tiles(), eight_thousand.height_in_tiles()), (32, 32));
        assert_eq!(eight_thousand.count(), 1024);
        let odd = TileRange::whole_canvas(300, 300).unwrap();
        assert_eq!(odd.count(), 4);
        let max = TileRange::whole_canvas(32_768, 32_768).unwrap();
        assert_eq!(max.count(), 128 * 128);
        assert!(TileRange::whole_canvas(0, 10).is_none());
    }

    #[test]
    fn coords_are_row_major_and_complete() {
        let range = TileRange { x0: 1, y0: 2, x1: 3, y1: 4 };
        let all: Vec<_> = range.coords().collect();
        assert_eq!(
            all,
            vec![TileCoord::new(1, 2), TileCoord::new(2, 2), TileCoord::new(1, 3), TileCoord::new(2, 3)]
        );
        assert_eq!(all.len() as u64, range.count());
        assert!(all.iter().all(|t| range.contains(*t)));
        assert!(!range.contains(TileCoord::new(3, 2)));
        assert!(!range.contains(TileCoord::new(0, 2)));
    }

    #[test]
    fn covering_is_exact_for_random_rectangles_near_tile_boundaries() {
        let mut rng = Lcg(12345);
        for _ in 0..2000 {
            let x = rng.range(230, 300) as u32;
            let y = rng.range(230, 300) as u32;
            let (w, h) = (rng.range(1, 40) as u32, rng.range(1, 40) as u32);
            let rect = ClippedRect { x, y, width: w, height: h };
            let range = TileRange::covering(rect).unwrap();
            let mut brute = HashSet::new();
            for px in x..x + w {
                for py in y..y + h {
                    brute.insert(TileCoord::locate(px, py).0);
                }
            }
            let from_range: HashSet<_> = range.coords().collect();
            assert_eq!(brute, from_range, "rect {x},{y} {w}x{h}");
        }
    }
}
