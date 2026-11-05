/// The underlying SPI and GPIO error values are preserved so callers can
/// inspect the original hardware error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<SpiError, PinError> {
    /// An SPI communication operation failed.
    Spi(SpiError),

    /// A GPIO operation failed.
    Pin(PinError),

    /// The display remained busy for longer than the configured timeout.
    BusyTimeout,
}

/// Errors produced when constructing a framebuffer region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionError {
    /// The region is empty or exceeds the display bounds.
    Invalid,

    /// The region is not horizontally byte-aligned.
    Unaligned,
}

/// Errors produced by framebuffer operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameBufferError {
    /// The requested pixel coordinates are outside the display bounds.
    OutOfBounds,
}
