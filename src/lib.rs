#![no_std]

mod command;
mod driver;
mod error;
mod framebuffer;

pub use driver::EpdDisplay;
pub use error::EpdError;
pub use framebuffer::{BUFFER_SIZE, FrameBuffer, HEIGHT, ROW_BYTES, WIDTH};