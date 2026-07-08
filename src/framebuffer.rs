use crate::{
    command::Command,
    error::EpdError,
    driver::EpdDisplay::SPI,
};

pub const WIDTH: usize = 88;
pub const HEIGHT: usize = 184;
pub const ROW_BYTES: usize = WIDTH.div_ceil(8);
pub const BUFFER_SIZE: usize = ROW_BYTES * HEIGHT;

const WHITE: bool = true;
const BLACK: bool = false;

pub struct FrameBuffer {
    data: [u8; BUFFER_SIZE],
}

impl FrameBuffer {
    /// Creates a new framebuffer with all pixels set to white
    pub const fn new() -> Self {
        Self {
            data: [0xFF; BUFFER_SIZE],
        }
    }

    /// Clears the framebuffer to all white or all black
    pub fn clear(&mut self, white: bool) {
        self.data.fill(if white { 0xFF } else { 0x00 });
    }

    /// Returns a reference to the framebuffer data as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Sets a pixel in the framebuffer to black or white. Does nothing if the coordinates are out of bounds.
    pub fn set_pixel(&mut self, x: usize, y: usize, black: bool) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let index = y * ROW_BYTES + x / 8;
        let bit = 7 - (x % 8);

        if black {
            self.data[index] &= !(1 << bit);
        } else {
            self.data[index] |= 1 << bit;
        }
    }

    /// Gets the color of a pixel in the framebuffer. Returns true for white, false for black. Returns None if out of bounds.
    pub fn show_full<D: DelayNs>(
        &mut self,
        buffer: &[u8; BUFFER_SIZE],
        delay: &mut D,
    ) -> Result<(), EpdError<SPI::Error>> {
            self.set_full_ram_window()?;
            self.set_ram_cursor(0, 0)?;

            self.send_command(Command::WriteRam as u8)?;
            self.send_data(buffer)?;

            self.set_ram_cursor(0, 0)?;
            self.send_command(Command::WriteVcomRam as u8)?;
            self.send_data(buffer)?;

            self.refresh_full(delay)
    }

    /// Puts the display into deep sleep mode to save power. The display can be woken up by toggling the RST pin.
    pub fn sleep(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::DeepSleep as u8)?;
        self.send_data(&[0x01])?;
        Ok(())
    }
}