//! A `no_std` driver for the GDEM0097T61 monochrome e-paper display.
//!
//! The crate provides a platform-independent interface for an 88 × 184
//! black-and-white display using [`embedded-hal`] 1.0 traits. It includes a
//! packed one-bit [`FrameBuffer`], validated partial-update [`Region`] values,
//! configurable BUSY-pin polling, optional non-blocking refresh triggering,
//! and deep-sleep support.
//!
//! # Framebuffer usage
//!
//! ```
//! use epd_gdem0097t61::{Color, FrameBuffer};
//!
//! let mut framebuffer = FrameBuffer::new();
//! framebuffer.set_pixel(10, 20, Color::Black).unwrap();
//!
//! assert_eq!(
//!     framebuffer.color_of_pixel(10, 20),
//!     Ok(Color::Black)
//! );
//! assert_eq!(
//!     framebuffer.color_of_pixel(11, 20),
//!     Ok(Color::White)
//! );
//! ```
//!
//! # Full display update
//!
//! The target platform supplies the SPI device, GPIO pins, and delay provider.
//!
//! ```no_run
//! use embedded_hal::{
//!     delay::DelayNs,
//!     digital::{InputPin, OutputPin},
//!     spi::SpiDevice,
//! };
//! use epd_gdem0097t61::{Config, Display, Error, FrameBuffer};
//!
//! fn update_display<SPI, DC, RESET, BUSY, DELAY, PinError>(
//!     spi: SPI,
//!     pin_dc: DC,
//!     pin_reset: RESET,
//!     pin_busy: BUSY,
//!     delay: DELAY,
//! ) -> Result<(), Error<SPI::Error, PinError>>
//! where
//!     SPI: SpiDevice<u8>,
//!     DC: OutputPin<Error = PinError>,
//!     RESET: OutputPin<Error = PinError>,
//!     BUSY: InputPin<Error = PinError>,
//!     DELAY: DelayNs,
//! {
//!     let mut display = Display::new(
//!         spi,
//!         pin_dc,
//!         pin_reset,
//!         pin_busy,
//!         delay,
//!         Config::default(),
//!     );
//!
//!     let framebuffer = FrameBuffer::new();
//!     display.init()?;
//!     display.show_full(&framebuffer)?;
//!     Ok(())
//! }
//! ```
//!
//! # Non-blocking refresh trigger
//!
//! Setting [`Config::wait_after_refresh`] to `false` makes `show_full()` and
//! `show_partial()` return after the refresh command has been sent. Reset and
//! initialization still wait until the controller is ready.
//!
//! ```
//! use epd_gdem0097t61::Config;
//!
//! let config = Config {
//!     wait_after_refresh: false,
//!     ..Config::default()
//! };
//!
//! assert!(!config.wait_after_refresh);
//! ```
//!
//! # Partial display update
//!
//! Partial updates require a horizontally byte-aligned region. Both the X
//! coordinate and width must be divisible by eight.
//!
//! ```
//! use epd_gdem0097t61::Region;
//!
//! let region = Region::new(0, 20, 32, 16).unwrap();
//! assert_eq!(region.width(), 32);
//! ```
#![cfg_attr(not(test), no_std)]

mod command;
mod config;
mod driver;
mod error;
mod framebuffer;
mod graphics;

pub use config::Config;
pub use driver::Display;
pub use error::{Error, FrameBufferError, RegionError};
pub use framebuffer::{BUFFER_SIZE, Color, FrameBuffer, HEIGHT, ROW_BYTES, Region, WIDTH};
pub use graphics::LandscapeFrameBuffer;

#[cfg(test)]
mod tests;
