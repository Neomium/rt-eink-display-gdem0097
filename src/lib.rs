#![cfg_attr(not(test), no_std)]

mod command;
mod driver;
mod error;
mod framebuffer;

pub use driver::EpdDisplay;
pub use error::EpdError;
pub use framebuffer::{BUFFER_SIZE, Color, FrameBuffer, HEIGHT, ROW_BYTES, Region, WIDTH};

#[cfg(test)]
mod tests;
