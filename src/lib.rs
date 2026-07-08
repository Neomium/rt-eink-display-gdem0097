#![no_std]

mod command;
mod error;
mod framebuffer;
mod driver;

pub const WIDTH: usize = 88;
pub const HEIGHT: usize = 184;
pub const ROW_BYTES: usize = WIDTH.div_ceil(8);
pub const BUFFER_SIZE: usize = ROW_BYTES * HEIGHT;