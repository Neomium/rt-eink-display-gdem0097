use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

use crate::{
    command::Command,
    error::EpdError,
    framebuffer::{BUFFER_SIZE, FrameBuffer, HEIGHT, ROW_BYTES, Region, WIDTH},
};

/// Display-update control value used for a full refresh.
const FULL_REFRESH_SEQUENCE: u8 = 0xF7;

/// Display-update control value used for a partial refresh.
const PARTIAL_REFRESH_CONTROL: u8 = 0xFF;

/// Controller is busy for at most 15 seconds.
const BUSY_TIMEOUT_MS: u32 = 15_000;

/// Data entry mode: increment X and Y after each byte written.
const DATA_ENTRY_MODE: u8 = 0x03;

/// Border waveform configuration recommended by the display datasheet.
const BORDER_WAVEFORM: u8 = 0x05;

/// Use the built-in temperature sensor.
const TEMPERATURE_SENSOR_INTERNAL: u8 = 0x80;

/// Display update configuration used during initialization.
const DISPLAY_UPDATE_CONTROL1: [u8; 2] = [0x00, 0x80];

/// Hardware driver for the GDEM0097T61 e-paper display.
pub struct EpdDisplay<SPI, DC, RST, BUSY, DELAY> {
    spi: SPI,
    dc: DC,
    rst: RST,
    busy: BUSY,
    delay: DELAY,
}

impl<SPI, DC, RST, BUSY, DELAY> EpdDisplay<SPI, DC, RST, BUSY, DELAY>
where
    SPI: SpiDevice<u8>,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
    DELAY: DelayNs,
{
    /// Creates a display driver and initially deselects the display.
    /// # Errors
    ///
    /// Returns [`EpdError::Pin`] if the chip-select pin cannot be set high.
    pub fn new(
        spi: SPI,
        dc: DC,
        rst: RST,
        busy: BUSY,
        delay: DELAY,
    ) -> Self {
        Self {
            spi,
            dc,
            rst,
            busy,
            delay,
        }
    }

    /// Returns ownership of the SPI bus and all control pins.
    pub fn release(self) -> (SPI, DC, RST, BUSY, DELAY) {
        (self.spi, self.dc, self.rst, self.busy, self.delay)
    }

    /// Performs a hardware reset.
    ///
    /// # Errors
    ///
    /// Returns [`EpdError::Pin`] if the reset pin cannot be changed.
    pub fn reset(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.rst.set_high().map_err(|_| EpdError::Pin)?;
        self.delay.delay_ms(20);

        self.rst.set_low().map_err(|_| EpdError::Pin)?;
        self.delay.delay_ms(10);

        self.rst.set_high().map_err(|_| EpdError::Pin)?;
        self.delay.delay_ms(20);

        Ok(())
    }

    /// Initializes the display controller.
    ///
    /// This performs a hardware reset and configures the controller's RAM,
    /// waveform, temperature sensor, and address counters.
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`EpdError::Spi`] if an SPI transfer fails;
    /// - [`EpdError::Pin`] if a GPIO operation fails;
    /// - [`EpdError::BusyTimeout`] if the controller does not become idle.
    pub fn init(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.reset()?;

        self.send_command(Command::SoftwareReset)?;
        self.wait_until_idle(BUSY_TIMEOUT_MS)?;

        self.send_command(Command::DriverOutputControl)?;
        self.send_data(&[0xB7, 0x00, 0x00])?;

        self.send_command(Command::DataEntryModeSetting)?;
        self.send_data(&[DATA_ENTRY_MODE])?;

        self.set_full_ram_window()?;

        self.send_command(Command::BorderWaveformControl)?;
        self.send_data(&[BORDER_WAVEFORM])?;

        self.send_command(Command::TemperatureSensorControl)?;
        self.send_data(&[TEMPERATURE_SENSOR_INTERNAL])?;

        self.send_command(Command::DisplayUpdateControl1)?;
        self.send_data(&DISPLAY_UPDATE_CONTROL1)?;

        self.set_ram_cursor(0, 0)?;

        Ok(())
    }

    /// Writes the complete framebuffer and performs a full refresh.
    ///
    /// # Errors
    ///
    /// Returns an error if SPI or GPIO communication fails, or if the
    /// controller does not become idle before the timeout.
    pub fn show_full(&mut self, buffer: &[u8; BUFFER_SIZE]) -> Result<(), EpdError<SPI::Error>> {
        self.set_full_ram_window()?;
        self.set_ram_cursor(0, 0)?;

        self.send_command(Command::WriteRam)?;
        self.send_data(buffer)?;

        self.refresh_full()
    }

    //// Writes one byte-aligned framebuffer region and performs a partial refresh.
    ///
    /// The region's X coordinate and width must both be divisible by eight.
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`EpdError::InvalidRegion`] if the region is empty or out of bounds;
    /// - [`EpdError::UnalignedRegion`] if X or width is not byte-aligned;
    /// - an SPI, pin, or busy-timeout error if the update fails.
    pub fn show_partial(
        &mut self,
        framebuffer: &FrameBuffer,
        region: Region,
    ) -> Result<(), EpdError<SPI::Error>> {
        self.validate_region(region)?;
        self.set_partial_ram_window(region)?;

        let x_byte = u8::try_from(region.x / 8).map_err(|_| EpdError::InvalidRegion)?;

        let y = u16::try_from(region.y).map_err(|_| EpdError::InvalidRegion)?;

        self.set_ram_cursor(x_byte, y)?;
        self.write_partial_ram(framebuffer, region)?;
        self.refresh_partial()
    }

    /// Puts the display into deep sleep.
    ///
    /// The display must be reset and initialized before it can be used again.
    ///
    /// # Errors
    ///
    /// Returns [`EpdError::Spi`] or [`EpdError::Pin`] if the sleep command
    /// cannot be sent.
    pub fn sleep(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::DeepSleepMode)?;
        self.send_data(&[0x01])
    }

    /// Sends one command byte.
    fn send_command(&mut self, command: Command) -> Result<(), EpdError<SPI::Error>> {
        self.dc.set_low().map_err(|_| EpdError::Pin)?;
        //self.cs.set_low().map_err(|_| EpdError::Pin)?;

        self.spi.write(&[command as u8]).map_err(EpdError::Spi)

        //let write_result = self.spi.write(&[command as u8]);
        //let deselect_result = self.cs.set_high();

        //write_result.map_err(EpdError::Spi)?;
        //deselect_result.map_err(|_| EpdError::Pin)?;

        //Ok(())
    }

    /// Sends data bytes belonging to a command.
    fn send_data(&mut self, data: &[u8]) -> Result<(), EpdError<SPI::Error>> {
        self.dc.set_high().map_err(|_| EpdError::Pin)?;
        //self.cs.set_low().map_err(|_| EpdError::Pin)?;

        self.spi.write(data).map_err(EpdError::Spi)

        // let write_result = self.spi.write(data);
        //let deselect_result = self.cs.set_high();

        // write_result.map_err(EpdError::Spi)?;
        // deselect_result.map_err(|_| EpdError::Pin)?;

        //Ok(())
    }

    /// Triggers a complete display refresh.
    fn refresh_full(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::DisplayUpdateControl2)?;
        self.send_data(&[FULL_REFRESH_SEQUENCE])?;

        self.send_command(Command::MasterActivation)?;

        self.wait_until_idle(BUSY_TIMEOUT_MS)
    }

    /// Triggers a partial display refresh.
    fn refresh_partial(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::DisplayUpdateControl2)?;
        self.send_data(&[PARTIAL_REFRESH_CONTROL])?;

        self.send_command(Command::MasterActivation)?;

        self.wait_until_idle(BUSY_TIMEOUT_MS)
    }

    /// Waits until the display is ready or the timeout is reached.
    fn wait_until_idle(&mut self, timeout_ms: u32) -> Result<(), EpdError<SPI::Error>> {
        let mut elapsed_ms = 0;

        while self.busy.is_high().map_err(|_| EpdError::Pin)? {
            if elapsed_ms >= timeout_ms {
                return Err(EpdError::BusyTimeout);
            }

            self.delay.delay_ms(10);
            elapsed_ms += 10;
        }

        Ok(())
    }

    /// Validates a partial-update region.
    #[allow(clippy::unused_self)]
    fn validate_region(&self, region: Region) -> Result<(), EpdError<SPI::Error>> {
        if region.width == 0 || region.height == 0 {
            return Err(EpdError::InvalidRegion);
        }

        let Some(x_end) = region.x.checked_add(region.width) else {
            return Err(EpdError::InvalidRegion);
        };

        let Some(y_end) = region.y.checked_add(region.height) else {
            return Err(EpdError::InvalidRegion);
        };

        if x_end > WIDTH || y_end > HEIGHT {
            return Err(EpdError::InvalidRegion);
        }

        if region.x % 8 != 0 || region.width % 8 != 0 {
            return Err(EpdError::UnalignedRegion);
        }

        Ok(())
    }

    /// Selects the entire display RAM area.
    #[allow(clippy::cast_possible_truncation)]
    fn set_full_ram_window(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::SetRamXAddressStartEnd)?;
        self.send_data(&[0, (ROW_BYTES - 1) as u8])?;

        self.send_command(Command::SetRamYAddressStartEnd)?;
        self.send_data(&[0, 0, ((HEIGHT - 1) & 0xFF) as u8, ((HEIGHT - 1) >> 8) as u8])?;

        Ok(())
    }

    /// Selects a byte-aligned rectangular RAM area.
    #[allow(clippy::cast_possible_truncation)]
    fn set_partial_ram_window(&mut self, region: Region) -> Result<(), EpdError<SPI::Error>> {
        let x_start_byte = region.x / 8;
        let x_end_byte = (region.x + region.width) / 8 - 1;

        let y_start = region.y;
        let y_end = region.y + region.height - 1;

        self.send_command(Command::SetRamXAddressStartEnd)?;
        self.send_data(&[x_start_byte as u8, x_end_byte as u8])?;

        self.send_command(Command::SetRamYAddressStartEnd)?;
        self.send_data(&[
            (y_start & 0xFF) as u8,
            (y_start >> 8) as u8,
            (y_end & 0xFF) as u8,
            (y_end >> 8) as u8,
        ])?;

        Ok(())
    }

    /// Writes only the framebuffer bytes belonging to a region.
    fn write_partial_ram(
        &mut self,
        framebuffer: &FrameBuffer,
        region: Region,
    ) -> Result<(), EpdError<SPI::Error>> {
        let start_byte = region.x / 8;
        let bytes_per_row = region.width / 8;
        let framebuffer_bytes = framebuffer.as_bytes();

        self.send_command(Command::WriteRam)?;

        for y in region.y..region.y + region.height {
            let row_start = y * ROW_BYTES + start_byte;
            let row_end = row_start + bytes_per_row;

            self.send_data(&framebuffer_bytes[row_start..row_end])?;
        }

        Ok(())
    }

    /// Sets the display RAM write cursor.
    #[allow(clippy::cast_possible_truncation)]
    fn set_ram_cursor(&mut self, x_byte: u8, y: u16) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::SetRamXAddressCounter)?;
        self.send_data(&[x_byte])?;

        self.send_command(Command::SetRamYAddressCounter)?;
        self.send_data(&[(y & 0xFF) as u8, (y >> 8) as u8])?;

        Ok(())
    }
}
