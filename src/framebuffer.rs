use crate::{FrameBufferError, RegionError};

/// Display width in pixels.
pub const WIDTH: usize = 88;

/// Display height in pixels.
pub const HEIGHT: usize = 184;

/// Number of bytes required for one row.
pub const ROW_BYTES: usize = WIDTH.div_ceil(8);

/// Total framebuffer size in bytes.
pub const BUFFER_SIZE: usize = ROW_BYTES * HEIGHT;

/// A monochrome framebuffer pixel color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// A cleared framebuffer bit.
    Black,

    /// A set framebuffer bit.
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

/// A rectangular area of the display.
///
/// The X coordinate and width are always aligned to complete framebuffer
/// bytes. Instances can therefore only be created through [`Region::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// Horizontal start coordinate in pixels.
    pub(crate) x: usize,

    /// Vertical start coordinate in pixels.
    pub(crate) y: usize,

    /// Region width in pixels.
    pub(crate) width: usize,

    /// Region height in pixels.
    pub(crate) height: usize,
}

impl Region {
    /// Creates a valid byte-aligned display region.
    ///
    /// # Errors
    ///
    /// Returns [`RegionError::Invalid`] if the region is empty, overflows, or
    /// extends beyond the display bounds.
    ///
    /// Returns [`RegionError::Unaligned`] if `x` or `width` is not divisible
    /// by eight.
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Result<Self, RegionError> {
        if width == 0 || height == 0 {
            return Err(RegionError::Invalid);
        }

        if x % 8 != 0 || width % 8 != 0 {
            return Err(RegionError::Unaligned);
        }

        let Some(end_x) = x.checked_add(width) else {
            return Err(RegionError::Invalid);
        };

        let Some(end_y) = y.checked_add(height) else {
            return Err(RegionError::Invalid);
        };

        if end_x > WIDTH || end_y > HEIGHT {
            return Err(RegionError::Invalid);
        }

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    #[must_use]
    pub const fn x(&self) -> usize {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> usize {
        self.y
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) const fn x_byte(&self) -> u8 {
        (self.x / 8) as u8
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) const fn x_end_byte(&self) -> u8 {
        ((self.x + self.width) / 8 - 1) as u8
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) const fn y_u16(&self) -> u16 {
        self.y as u16
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) const fn y_end_u16(&self) -> u16 {
        (self.y + self.height - 1) as u16
    }

    pub(crate) const fn start_byte(&self) -> usize {
        self.x / 8
    }

    pub(crate) const fn bytes_per_row(&self) -> usize {
        self.width / 8
    }

    pub(crate) const fn end_y_exclusive(&self) -> usize {
        self.y + self.height
    }
}

/// One-bit-per-pixel framebuffer for the GDEM0097T61 display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameBuffer {
    data: [u8; BUFFER_SIZE],
}

impl FrameBuffer {
    /// Creates a framebuffer with every pixel set to white.
    #[must_use]
    pub const fn new() -> Self {
        Self::filled(Color::White)
    }

    /// Creates a framebuffer filled entirely with the given color.
    #[must_use]
    pub const fn filled(color: Color) -> Self {
        Self {
            data: [color.fill_byte(); BUFFER_SIZE],
        }
    }

    /// Fills the complete framebuffer with white or black.
    pub fn clear(&mut self, color: Color) {
        self.data.fill(color.fill_byte());
    }

    /// Returns the raw framebuffer bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; BUFFER_SIZE] {
        &self.data
    }

    /// Sets one pixel to black or white.
    ///
    /// # Errors
    ///
    /// Returns [`FrameBufferError::OutOfBounds`] when the coordinates are outside
    /// the display.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<(), FrameBufferError> {
        if x >= WIDTH || y >= HEIGHT {
            return Err(FrameBufferError::OutOfBounds);
        }

        let byte_index = y * ROW_BYTES + x / 8;
        let bit_index = 7 - (x % 8);
        let mask = 1u8 << bit_index;

        match color {
            Color::Black => self.data[byte_index] &= !mask,
            Color::White => self.data[byte_index] |= mask,
        }

        Ok(())
    }

    /// Returns the color of one pixel.
    ///
    /// # Errors
    ///
    /// Returns [`FrameBufferError::OutOfBounds`] if the coordinates are outside
    /// the display.
    pub fn color_of_pixel(&self, x: usize, y: usize) -> Result<Color, FrameBufferError> {
        if x >= WIDTH || y >= HEIGHT {
            return Err(FrameBufferError::OutOfBounds);
        }

        let byte_index = y * ROW_BYTES + x / 8;
        let bit_index = 7 - (x % 8);
        let mask = 1u8 << bit_index;

        if self.data[byte_index] & mask == 0 {
            Ok(Color::Black)
        } else {
            Ok(Color::White)
        }
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}
