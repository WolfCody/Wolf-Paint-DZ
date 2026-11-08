//! One raster tile: 256 x 256 pixels of RGBA8 with premultiplied alpha.

use std::fmt;

/// Edge length of a tile in pixels. `core-model` keeps the same constant for its tile-grid arithmetic
/// (`scripts/check_project_consistency.py` fails if the two ever differ).
pub const TILE_SIZE: u32 = 256;
pub const BYTES_PER_PIXEL: usize = 4;
pub const TILE_PIXELS: usize = (TILE_SIZE as usize) * (TILE_SIZE as usize);
/// 262,144 bytes (256 KiB) per tile.
pub const TILE_BYTES: usize = TILE_PIXELS * BYTES_PER_PIXEL;
const ROW_BYTES: usize = (TILE_SIZE as usize) * BYTES_PER_PIXEL;

/// One pixel: red, green, blue, alpha. Premultiplied: the color channels are already multiplied by alpha.
pub type Rgba = [u8; 4];
pub const TRANSPARENT: Rgba = [0, 0, 0, 0];

/// Premultiplies a straight-alpha color with rounding: `c * a / 255`, rounded to nearest.
pub fn premultiply(straight: Rgba) -> Rgba {
    let alpha = u16::from(straight[3]);
    // 255 * 255 + 127 = 65,152 fits into u16
    let scale = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
    [scale(straight[0]), scale(straight[1]), scale(straight[2]), straight[3]]
}

/// A tile owns exactly [`TILE_BYTES`] bytes on the heap. It is never built on the stack: a 256 KiB temporary
/// would overflow the small stacks of Android worker threads.
#[derive(Clone, PartialEq, Eq)]
pub struct Tile {
    data: Box<[u8]>,
}

impl Tile {
    /// A fully transparent tile (all bytes zero).
    pub fn transparent() -> Tile {
        Tile {
            data: vec![0u8; TILE_BYTES].into_boxed_slice(),
        }
    }

    /// Builds a tile from exactly [`TILE_BYTES`] bytes (for example read from disk); `None` for any other length.
    pub fn from_bytes(bytes: &[u8]) -> Option<Tile> {
        if bytes.len() == TILE_BYTES {
            Some(Tile { data: bytes.into() })
        } else {
            None
        }
    }

    fn index(x: u32, y: u32) -> Option<usize> {
        if x < TILE_SIZE && y < TILE_SIZE {
            Some(((y as usize) * (TILE_SIZE as usize) + (x as usize)) * BYTES_PER_PIXEL)
        } else {
            None
        }
    }

    /// Pixel at tile-local coordinates; `None` outside 0..256.
    pub fn get(&self, x: u32, y: u32) -> Option<Rgba> {
        let i = Self::index(x, y)?;
        Some([self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]])
    }

    /// Writes a pixel; `false` (and no change) outside 0..256.
    pub fn set(&mut self, x: u32, y: u32, color: Rgba) -> bool {
        match Self::index(x, y) {
            Some(i) => {
                self.data[i..i + BYTES_PER_PIXEL].copy_from_slice(&color);
                true
            }
            None => false,
        }
    }

    pub fn fill(&mut self, color: Rgba) {
        for pixel in self.data.chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&color);
        }
    }

    /// All pixels, row by row, 4 bytes per pixel.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// One pixel row (1024 bytes); `None` for `y >= 256`.
    pub fn row(&self, y: u32) -> Option<&[u8]> {
        if y < TILE_SIZE {
            let start = (y as usize) * ROW_BYTES;
            Some(&self.data[start..start + ROW_BYTES])
        } else {
            None
        }
    }

    /// True when every pixel has alpha 0 (in premultiplied data that means the pixel is empty).
    pub fn is_transparent(&self) -> bool {
        self.data.chunks_exact(BYTES_PER_PIXEL).all(|pixel| pixel[3] == 0)
    }
}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tile {{ {TILE_SIZE}x{TILE_SIZE} RGBA8 premultiplied }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_the_documented_sizes() {
        assert_eq!(TILE_SIZE, 256);
        assert_eq!(TILE_PIXELS, 65_536);
        assert_eq!(TILE_BYTES, 262_144);
    }

    #[test]
    fn a_new_tile_is_fully_transparent() {
        let tile = Tile::transparent();
        assert_eq!(tile.as_bytes().len(), TILE_BYTES);
        assert!(tile.is_transparent());
        assert_eq!(tile.get(0, 0), Some(TRANSPARENT));
        assert_eq!(tile.get(255, 255), Some(TRANSPARENT));
    }

    #[test]
    fn set_and_get_round_trip_at_the_corners_without_touching_neighbours() {
        let mut tile = Tile::transparent();
        let corners = [(0, 0, [1, 2, 3, 255]), (255, 0, [4, 5, 6, 255]), (0, 255, [7, 8, 9, 255]), (255, 255, [10, 11, 12, 255])];
        for (x, y, color) in corners {
            assert!(tile.set(x, y, color));
        }
        for (x, y, color) in corners {
            assert_eq!(tile.get(x, y), Some(color));
        }
        assert_eq!(tile.get(1, 0), Some(TRANSPARENT));
        assert_eq!(tile.get(254, 255), Some(TRANSPARENT));
        assert_eq!(tile.get(128, 128), Some(TRANSPARENT));
    }

    #[test]
    fn out_of_range_access_is_rejected_without_changes() {
        let mut tile = Tile::transparent();
        assert_eq!(tile.get(256, 0), None);
        assert_eq!(tile.get(0, 256), None);
        assert_eq!(tile.get(u32::MAX, u32::MAX), None);
        assert!(!tile.set(256, 0, [1, 1, 1, 255]));
        assert!(!tile.set(0, 256, [1, 1, 1, 255]));
        assert!(tile.is_transparent());
    }

    #[test]
    fn transparency_follows_alpha() {
        let mut tile = Tile::transparent();
        tile.set(10, 20, [0, 0, 0, 1]);
        assert!(!tile.is_transparent());
        tile.set(10, 20, TRANSPARENT);
        assert!(tile.is_transparent());
    }

    #[test]
    fn fill_and_rows() {
        let mut tile = Tile::transparent();
        tile.fill([9, 8, 7, 255]);
        assert_eq!(tile.get(100, 100), Some([9, 8, 7, 255]));
        assert!(!tile.is_transparent());
        let row = tile.row(255).unwrap();
        assert_eq!(row.len(), 1024);
        assert_eq!(&row[..4], &[9, 8, 7, 255]);
        assert!(tile.row(256).is_none());
        let mut other = Tile::transparent();
        other.set(3, 4, [1, 2, 3, 4]);
        assert_eq!(&other.row(4).unwrap()[12..16], &[1, 2, 3, 4]);
    }

    #[test]
    fn from_bytes_requires_the_exact_length() {
        assert!(Tile::from_bytes(&[0u8; 10]).is_none());
        assert!(Tile::from_bytes(&vec![0u8; TILE_BYTES + 1]).is_none());
        let mut bytes = vec![0u8; TILE_BYTES];
        bytes[0..4].copy_from_slice(&[5, 6, 7, 8]);
        let tile = Tile::from_bytes(&bytes).unwrap();
        assert_eq!(tile.get(0, 0), Some([5, 6, 7, 8]));
    }

    #[test]
    fn clones_are_independent() {
        let mut a = Tile::transparent();
        a.set(1, 1, [1, 1, 1, 255]);
        let b = a.clone();
        a.set(1, 1, [2, 2, 2, 255]);
        assert_eq!(b.get(1, 1), Some([1, 1, 1, 255]));
        assert_eq!(a.get(1, 1), Some([2, 2, 2, 255]));
    }

    #[test]
    fn premultiply_rounds_to_nearest_and_never_exceeds_alpha() {
        assert_eq!(premultiply([255, 0, 0, 255]), [255, 0, 0, 255]);
        assert_eq!(premultiply([255, 255, 255, 0]), [0, 0, 0, 0]);
        assert_eq!(premultiply([200, 100, 50, 128]), [100, 50, 25, 128]);
        for alpha in 0..=255u8 {
            for channel in 0..=255u8 {
                let p = premultiply([channel, channel, channel, alpha]);
                assert!(p[0] <= alpha, "channel {channel} alpha {alpha} -> {}", p[0]);
            }
        }
    }

    #[test]
    fn debug_output_does_not_dump_the_pixels() {
        let text = format!("{:?}", Tile::transparent());
        assert!(text.len() < 80, "{text}");
        assert!(text.contains("256x256"));
    }

    #[test]
    fn a_tile_is_created_on_the_heap_even_on_a_tiny_stack() {
        // 64 KiB of stack: a 256 KiB array temporary would overflow it
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(|| {
                let tile = Tile::transparent();
                tile.is_transparent()
            })
            .unwrap();
        assert!(handle.join().unwrap());
    }

    #[test]
    fn tile_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Tile>();
    }
}
