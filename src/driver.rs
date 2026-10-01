use embedded_hal::digital::{InputPin, OutputPin};

#[cfg(not(feature = "async"))]
use embedded_hal::{delay::DelayNs, spi::SpiDevice};

#[cfg(feature = "async")]
use embedded_hal_async::{delay::DelayNs, spi::SpiDevice};

use crate::{
    command::Command,
    config::Config,
    error::Error,
    framebuffer::{FrameBuffer, HEIGHT, ROW_BYTES, Region},
};

/// Time for which the hardware reset pin is held low.
const RESET_LOW_TIME_US: u32 = 200;

/// Time allowed after releasing the hardware reset pin.
const RESET_RECOVERY_TIME_US: u32 = 200;

/// Display-update control value used for a full refresh.
const FULL_REFRESH_SEQUENCE: u8 = 0xF7;

/// Display-update control value used for a partial refresh.
const PARTIAL_REFRESH_CONTROL: u8 = 0xFF;

/// Data entry mode: increment X and Y after each byte written.
const DATA_ENTRY_MODE: u8 = 0x03;

/// Border waveform configuration recommended by the display datasheet.
const BORDER_WAVEFORM: u8 = 0x05;

/// Use the built-in temperature sensor.
const TEMPERATURE_SENSOR_INTERNAL: u8 = 0x80;

/// Display update configuration used during initialization.
const DISPLAY_UPDATE_CONTROL1: [u8; 2] = [0x00, 0x80];

/// Hardware driver for the GDEM0097T61 e-paper display.
pub struct Display<SPI, DC, RST, BUSY, DELAY> {
    /// SPI device used to communicate with the display controller.
    spi: SPI,

    /// Data/command pin used to distinguish commands from display data.
    pin_dc: DC,

    /// Reset pin used to perform a hardware reset of the display.
    pin_reset: RST,

    /// Busy pin used to determine when the display controller is ready.
    pin_busy: BUSY,

    /// Delay provider used for reset timing and busy polling.
    delay: DELAY,

    /// Runtime timing and refresh configuration.
    config: Config,
}

impl<SPI, DC, RST, BUSY, DELAY, PinError> Display<SPI, DC, RST, BUSY, DELAY>
where
    SPI: SpiDevice<u8>,
    DC: OutputPin<Error = PinError>,
    RST: OutputPin<Error = PinError>,
    BUSY: InputPin<Error = PinError>,
    DELAY: DelayNs,
{
    /// Creates a display driver with the supplied interface and configuration.
    pub fn new(
        spi: SPI,
        pin_dc: DC,
        pin_reset: RST,
        pin_busy: BUSY,
        delay: DELAY,
        config: Config,
    ) -> Self {
        Self {
            spi,
            pin_dc,
            pin_reset,
            pin_busy,
            delay,
            config,
        }
    }

    /// Returns ownership of the SPI device, pins, and delay provider.
    pub fn release(self) -> (SPI, DC, RST, BUSY, DELAY) {
        (
            self.spi,
            self.pin_dc,
            self.pin_reset,
            self.pin_busy,
            self.delay,
        )
    }

    /// Performs a hardware reset and waits until the controller is ready.
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`Error::Pin`] if the reset or BUSY pin operation fails;
    /// - [`Error::BusyTimeout`] if the display does not become idle before
    ///   the configured timeout.
    #[cfg(not(feature = "async"))]
    pub fn reset(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.pin_reset.set_low().map_err(Error::Pin)?;
        self.delay.delay_us(RESET_LOW_TIME_US);

        self.pin_reset.set_high().map_err(Error::Pin)?;
        self.delay.delay_us(RESET_RECOVERY_TIME_US);

        self.wait_until_idle()
    }

    #[cfg(feature = "async")]
    pub async fn reset(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.pin_reset.set_low().map_err(Error::Pin)?;
        self.delay.delay_us(RESET_LOW_TIME_US).await;
        self.pin_reset.set_high().map_err(Error::Pin)?;
        self.delay.delay_us(RESET_RECOVERY_TIME_US).await;
        self.wait_until_idle().await
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
    /// - [`Error::Spi`] if an SPI transfer fails;
    /// - [`Error::Pin`] if a GPIO operation fails;
    /// - [`Error::BusyTimeout`] if the controller does not become idle.
    #[cfg(not(feature = "async"))]
    pub fn init(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.reset()?;

        self.send_command(Command::SoftwareReset)?;
        self.wait_until_idle()?;

        self.send_command(Command::DriverOutputControl)?;
        // Configure 184 gate lines (184 - 1 = 0x00B7) with default scan settings.
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

        self.set_ram_cursor(0, 0)
    }

    #[cfg(feature = "async")]
    pub async fn init(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.reset().await?;
        self.send_command(Command::SoftwareReset).await?;
        self.wait_until_idle().await?;
        self.send_command(Command::DriverOutputControl).await?;
        self.send_data(&[0xB7, 0x00, 0x00]).await?;
        self.send_command(Command::DataEntryModeSetting).await?;
        self.send_data(&[DATA_ENTRY_MODE]).await?;
        self.set_full_ram_window().await?;
        self.send_command(Command::BorderWaveformControl).await?;
        self.send_data(&[BORDER_WAVEFORM]).await?;
        self.send_command(Command::TemperatureSensorControl).await?;
        self.send_data(&[TEMPERATURE_SENSOR_INTERNAL]).await?;
        self.send_command(Command::DisplayUpdateControl1).await?;
        self.send_data(&DISPLAY_UPDATE_CONTROL1).await?;
        self.set_ram_cursor(0, 0).await
    }

    /// Writes the complete framebuffer and performs a full refresh.
    ///
    /// When [`Config::wait_after_refresh`] is `false`, this method returns
    /// immediately after triggering the refresh.
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`Error::Spi`] if an SPI transfer fails;
    /// - [`Error::Pin`] if a GPIO operation fails;
    /// - [`Error::BusyTimeout`] if refresh waiting is enabled and the display
    ///   does not become idle before the configured timeout.
    #[cfg(not(feature = "async"))]
    pub fn show_full(
        &mut self,
        framebuffer: &FrameBuffer,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.set_full_ram_window()?;
        self.set_ram_cursor(0, 0)?;

        self.send_command(Command::WriteRam)?;
        self.send_data(framebuffer.as_bytes())?;

        self.refresh_full()
    }

    #[cfg(feature = "async")]
    pub async fn show_full(
        &mut self,
        framebuffer: &FrameBuffer,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.set_full_ram_window().await?;
        self.set_ram_cursor(0, 0).await?;
        self.send_command(Command::WriteRam).await?;
        self.send_data(framebuffer.as_bytes()).await?;
        self.refresh_full().await
    }

    /// Writes one byte-aligned framebuffer region and performs a partial refresh.
    ///
    /// Create regions with [`Region::new`]. When
    /// [`Config::wait_after_refresh`] is `false`, this method returns
    /// immediately after triggering the refresh.    
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`Error::Spi`] if an SPI transfer fails;
    /// - [`Error::Pin`] if a GPIO operation fails;
    /// - [`Error::BusyTimeout`] if refresh waiting is enabled and the display
    ///   does not become idle before the configured timeout.
    #[cfg(not(feature = "async"))]
    pub fn show_partial(
        &mut self,
        framebuffer: &FrameBuffer,
        region: Region,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.set_partial_ram_window(region)?;
        self.set_ram_cursor(region.x_byte(), region.y_u16())?;
        self.write_partial_ram(framebuffer, region)?;
        self.refresh_partial()
    }

    #[cfg(feature = "async")]
    pub async fn show_partial(
        &mut self,
        framebuffer: &FrameBuffer,
        region: Region,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.set_partial_ram_window(region).await?;
        self.set_ram_cursor(region.x_byte(), region.y_u16()).await?;
        self.write_partial_ram(framebuffer, region).await?;
        self.refresh_partial().await
    }

    /// Puts the display into deep sleep.
    ///
    /// The display must be reset and initialized before it can be used again.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Spi`] or [`Error::Pin`] if the sleep command
    /// cannot be sent.
    #[cfg(not(feature = "async"))]
    pub fn sleep(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::DeepSleepMode)?;
        self.send_data(&[0x01])
    }

    #[cfg(feature = "async")]
    pub async fn sleep(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::DeepSleepMode).await?;
        self.send_data(&[0x01]).await
    }

    #[cfg(not(feature = "async"))]
    fn send_command(&mut self, command: Command) -> Result<(), Error<SPI::Error, PinError>> {
        self.pin_dc.set_low().map_err(Error::Pin)?;
        self.spi.write(&[command as u8]).map_err(Error::Spi)
    }
    #[cfg(feature = "async")]
    async fn send_command(&mut self, command: Command) -> Result<(), Error<SPI::Error, PinError>> {
        self.pin_dc.set_low().map_err(Error::Pin)?;
        self.spi.write(&[command as u8]).await.map_err(Error::Spi)
    }

    #[cfg(not(feature = "async"))]
    fn send_data(&mut self, data: &[u8]) -> Result<(), Error<SPI::Error, PinError>> {
        self.pin_dc.set_high().map_err(Error::Pin)?;
        self.spi.write(data).map_err(Error::Spi)
    }

    #[cfg(feature = "async")]
    async fn send_data(&mut self, data: &[u8]) -> Result<(), Error<SPI::Error, PinError>> {
        self.pin_dc.set_high().map_err(Error::Pin)?;
        self.spi.write(data).await.map_err(Error::Spi)
    }

    /// Triggers a complete display refresh.
    #[cfg(not(feature = "async"))]
    fn refresh_full(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::DisplayUpdateControl2)?;
        self.send_data(&[FULL_REFRESH_SEQUENCE])?;

        self.send_command(Command::MasterActivation)?;

        self.wait_after_refresh_if_configured()
    }
    #[cfg(feature = "async")]
    async fn refresh_full(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::DisplayUpdateControl2).await?;
        self.send_data(&[FULL_REFRESH_SEQUENCE]).await?;

        self.send_command(Command::MasterActivation).await?;

        self.wait_after_refresh_if_configured().await
    }

    /// Triggers a partial display refresh.
    #[cfg(not(feature = "async"))]
    fn refresh_partial(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::DisplayUpdateControl2)?;
        self.send_data(&[PARTIAL_REFRESH_CONTROL])?;
        self.send_command(Command::MasterActivation)?;
        self.wait_after_refresh_if_configured()
    }

    #[cfg(feature = "async")]
    async fn refresh_partial(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::DisplayUpdateControl2).await?;
        self.send_data(&[PARTIAL_REFRESH_CONTROL]).await?;
        self.send_command(Command::MasterActivation).await?;
        self.wait_after_refresh_if_configured().await
    }

    #[cfg(not(feature = "async"))]
    fn wait_after_refresh_if_configured(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        if self.config.wait_after_refresh {
            self.wait_until_idle()?;
        }

        Ok(())
    }

    #[cfg(feature = "async")]
    async fn wait_after_refresh_if_configured(
        &mut self,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        if self.config.wait_after_refresh {
            self.wait_until_idle().await?;
        }
        Ok(())
    }

    /// Waits until the display is ready or configured timeout is reached.
    #[cfg(not(feature = "async"))]
    fn wait_until_idle(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        let mut elapsed_us = 0u32;
        let poll_interval_us = self.config.busy_poll_interval_us.max(1);

        while self.pin_busy.is_high().map_err(Error::Pin)? {
            if elapsed_us >= self.config.busy_timeout_us {
                return Err(Error::BusyTimeout);
            }

            self.delay.delay_us(poll_interval_us);
            elapsed_us = elapsed_us.saturating_add(poll_interval_us);
        }

        Ok(())
    }

    #[cfg(feature = "async")]
    async fn wait_until_idle(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        let mut elapsed_us = 0u32;
        let poll_interval_us = self.config.busy_poll_interval_us.max(1);
        while self.pin_busy.is_high().map_err(Error::Pin)? {
            if elapsed_us >= self.config.busy_timeout_us {
                return Err(Error::BusyTimeout);
            }
            self.delay.delay_us(poll_interval_us).await;
            elapsed_us = elapsed_us.saturating_add(poll_interval_us);
        }
        Ok(())
    }

    /// Selects the entire display RAM area.
    #[allow(clippy::cast_possible_truncation)]
    #[cfg(not(feature = "async"))]
    fn set_full_ram_window(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::SetRamXAddressStartEnd)?;
        self.send_data(&[0, (ROW_BYTES - 1) as u8])?;

        self.send_command(Command::SetRamYAddressStartEnd)?;
        self.send_data(&[0, 0, ((HEIGHT - 1) & 0xFF) as u8, ((HEIGHT - 1) >> 8) as u8])
    }

    #[cfg(feature = "async")]
    #[allow(clippy::cast_possible_truncation)]
    async fn set_full_ram_window(&mut self) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::SetRamXAddressStartEnd).await?;
        self.send_data(&[0, (ROW_BYTES - 1) as u8]).await?;
        self.send_command(Command::SetRamYAddressStartEnd).await?;
        self.send_data(&[0, 0, ((HEIGHT - 1) & 0xFF) as u8, ((HEIGHT - 1) >> 8) as u8])
            .await
    }

    #[cfg(not(feature = "async"))]
    fn set_partial_ram_window(
        &mut self,
        region: Region,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::SetRamXAddressStartEnd)?;
        self.send_data(&[region.x_byte(), region.x_end_byte()])?;

        let y_start = region.y_u16().to_le_bytes();
        let y_end = region.y_end_u16().to_le_bytes();

        self.send_command(Command::SetRamYAddressStartEnd)?;
        self.send_data(&[y_start[0], y_start[1], y_end[0], y_end[1]])
    }

    #[cfg(feature = "async")]
    async fn set_partial_ram_window(
        &mut self,
        region: Region,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::SetRamXAddressStartEnd).await?;
        self.send_data(&[region.x_byte(), region.x_end_byte()])
            .await?;
        let y_start = region.y_u16().to_le_bytes();
        let y_end = region.y_end_u16().to_le_bytes();
        self.send_command(Command::SetRamYAddressStartEnd).await?;
        self.send_data(&[y_start[0], y_start[1], y_end[0], y_end[1]])
            .await
    }

    /// Writes only the framebuffer bytes belonging to a region.
    #[cfg(not(feature = "async"))]
    fn write_partial_ram(
        &mut self,
        framebuffer: &FrameBuffer,
        region: Region,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        let framebuffer_bytes = framebuffer.as_bytes();

        self.send_command(Command::WriteRam)?;

        for y in region.y()..region.end_y_exclusive() {
            let row_start = y * ROW_BYTES + region.start_byte();
            let row_end = row_start + region.bytes_per_row();
            self.send_data(&framebuffer_bytes[row_start..row_end])?;
        }

        Ok(())
    }

    #[cfg(feature = "async")]
    async fn write_partial_ram(
        &mut self,
        framebuffer: &FrameBuffer,
        region: Region,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        let framebuffer_bytes = framebuffer.as_bytes();
        self.send_command(Command::WriteRam).await?;
        for y in region.y()..region.end_y_exclusive() {
            let row_start = y * ROW_BYTES + region.start_byte();
            let row_end = row_start + region.bytes_per_row();
            self.send_data(&framebuffer_bytes[row_start..row_end])
                .await?;
        }
        Ok(())
    }

    /// Sets the display RAM write cursor.
    #[allow(clippy::cast_possible_truncation)]
    #[cfg(not(feature = "async"))]
    fn set_ram_cursor(&mut self, x_byte: u8, y: u16) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::SetRamXAddressCounter)?;
        self.send_data(&[x_byte])?;

        self.send_command(Command::SetRamYAddressCounter)?;
        self.send_data(&[(y & 0xFF) as u8, (y >> 8) as u8])
    }

    #[cfg(feature = "async")]
    #[allow(clippy::cast_possible_truncation)]
    async fn set_ram_cursor(
        &mut self,
        x_byte: u8,
        y: u16,
    ) -> Result<(), Error<SPI::Error, PinError>> {
        self.send_command(Command::SetRamXAddressCounter).await?;
        self.send_data(&[x_byte]).await?;
        self.send_command(Command::SetRamYAddressCounter).await?;
        self.send_data(&[(y & 0xFF) as u8, (y >> 8) as u8]).await
    }
}
