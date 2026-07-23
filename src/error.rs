///
/// `SpiError` is generic because different SPI implementations
/// can have different error types. The driver does not need to know
/// the exact SPI error type in advance; it just wraps it inside
/// `EpdError::Spi`.
///
/// Pin errors from GPIO operations are simplified into `EpdError::Pin`.
#[derive(Debug)]
pub enum EpdError<SpiError> {
    /// An SPI communication operation failed.
    Spi(SpiError),

    /// A GPIO operation failed.
    Pin,

    /// The display remained busy for longer than the configured timeout.
    BusyTimeout,

    /// A partial-update region was empty, outside the display, or overflowed.
    InvalidRegion,

    /// A partial-update region was not aligned to whole framebuffer bytes.
    UnalignedRegion,
}
