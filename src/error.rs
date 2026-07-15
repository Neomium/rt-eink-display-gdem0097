///
/// `SpiError` is generic because different SPI implementations
/// can have different error types. The driver does not need to know
/// the exact SPI error type in advance; it just wraps it inside
/// `EpdError::Spi`.
///
/// Pin errors from GPIO operations are simplified into `EpdError::Pin`.
#[derive(Debug)]
pub enum EpdError<SpiError> {
    Spi(SpiError),
    Pin,
    BusyTimeout,
    InvalidRegion,
    UnalignedRegion,
}
