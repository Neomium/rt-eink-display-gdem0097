/// Display width in pixels.
pub const WIDTH: usize = 88;

/// Display height in pixels.
pub const HEIGHT: usize = 184;

/// Number of bytes required for one row.
pub const ROW_BYTES: usize = WIDTH.div_ceil(8);

/// Total framebuffer size in bytes.
pub const BUFFER_SIZE: usize = ROW_BYTES * HEIGHT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Black,
    White,
}

impl Color {
    const fn fill_byte(self) -> u8 {
        match self {
            Self::Black => 0x00,
            Self::White => 0xFF,
        }
    }
}

/// One-bit-per-pixel framebuffer for the GDEM0097T61 display.
///
/// A cleared bit represents black.
/// A set bit represents white.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameBuffer {
    data: [u8; BUFFER_SIZE],
}

impl FrameBuffer {
    /// Creates a framebuffer with every pixel set to white.
    pub const fn new() -> Self {
        Self {
            data: [0xFF; BUFFER_SIZE],
        }
    }

    /// Fills the complete framebuffer with white or black.
    pub fn clear(&mut self, color: Color) {
        self.data.fill(color.fill_byte());
    }

    /// Returns the raw framebuffer bytes.
    pub fn as_bytes(&self) -> &[u8; BUFFER_SIZE] {
        &self.data
    }

    /// Sets one pixel to black or white.
    ///
    /// Coordinates outside the display are ignored.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let byte_index = y * ROW_BYTES + x / 8;
        let bit_index = 7 - (x % 8);
        let mask = 1u8 << bit_index;

        match color {
            Color::Black => self.data[byte_index] &= !mask,
            Color::White => self.data[byte_index] |= mask,
        }
    }

    /// Returns the colour of one pixel.
    ///
    /// Returns `None` when the coordinates are outside the display.
      pub fn pixel(&self, x: usize, y: usize) -> Option<Color> {
        if x >= WIDTH || y >= HEIGHT {
            return None;
        }

        let byte_index = y * ROW_BYTES + x / 8;
        let bit_index = 7 - (x % 8);
        let mask = 1u8 << bit_index;

        if self.data[byte_index] & mask == 0 {
            Some(Color::Black)
        } else {
            Some(Color::White)
        }
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}