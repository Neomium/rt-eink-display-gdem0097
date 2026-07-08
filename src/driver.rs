// use embedded_hal::{
//     delay::DelayNs,
//     digital::{InputPin, OutputPin},
//     spi::SpiBus,
// };
// use core::fmt::Debug;
// use crate::error::EpdError;

// #[derive(Debug)]
// pub struct EpdDisplay<SPI, CS, DC, RST, BUSY> {
//     spi: SPI,
//     cs: CS,
//     dc: DC,
//     rst: RST,
//     busy: BUSY,
// }

// impl<SPI, CS, DC, RST, BUSY> EpdDisplay<SPI, CS, DC, RST, BUSY>
// where
//     SPI: SpiBus<u8>,
//     CS: OutputPin,
//     DC: OutputPin,
//     RST: OutputPin,
//     BUSY: InputPin,
// {

//     pub fn new(spi: SPI, cs: CS, dc: DC, rst: RST, busy: BUSY) -> Self {
//         Self {
//             spi,
//             cs,
//             dc,
//             rst,
//             busy,
//         }
//     }

//     pub fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), EpdError<SPI::Error>> {
//         self.rst.set_low().map_err(|_| EpdError::Pin)?;
//         delay.delay_ms(10);
//         self.rst.set_high().map_err(|_| EpdError::Pin)?;
//         delay.delay_ms(10);
//         Ok(())
//     }

//     pub fn send_command(&mut self, command: u8) -> Result<(), EpdError<SPI::Error>> {
//         self.dc.set_low().map_err(|_| EpdError::Pin)?;
//         self.cs.set_low().map_err(|_| EpdError::Pin)?;
//         self.spi.write(&[command]).map_err(EpdError::Spi)?;
//         self.cs.set_high().map_err(|_| EpdError::Pin)?;
//         Ok(())
//     }

//     pub fn send_data(&mut self, data: &[u8]) -> Result<(), EpdError<SPI::Error>> {
//         self.dc.set_high().map_err(|_| EpdError::Pin)?;
//         self.cs.set_low().map_err(|_| EpdError::Pin)?;
//         self.spi.write(data).map_err(EpdError::Spi)?;
//         self.cs.set_high().map_err(|_| EpdError::Pin)?;
//         Ok(())
//     }
// }

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiBus,
};

use crate::{
    command::Command,
    error::EpdError, 
    BUFFER_SIZE, HEIGHT, ROW_BYTES,
};

pub struct EpdDisplay<SPI, CS, DC, RST, BUSY> {
    spi: SPI,
    cs: CS,
    dc: DC,
    rst: RST,
    busy: BUSY,
}

impl<SPI, CS, DC, RST, BUSY> EpdDisplay<SPI, CS, DC, RST, BUSY>
where
    SPI: SpiBus<u8>,
    CS: OutputPin,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
{

    /// Creates the display driver and sets CS high so the display is initially deselected
    pub fn new(
        spi: SPI,
        mut cs: CS,
        dc: DC,
        rst: RST,
        busy: BUSY,
    ) -> Result<Self, EpdError<SPI::Error>> {
        cs.set_high().map_err(|_| EpdError::Pin)?;

        Ok(Self {
            spi,
            cs,
            dc,
            rst,
            busy,
        })
    }

    /// Gives back ownership of SPI and all pins
    pub fn release(self) -> (SPI, CS, DC, RST, BUSY) {
        (self.spi, self.cs, self.dc, self.rst, self.busy)
    }

    /// Resets the display by toggling the RST pin low for 10ms, then high for 10ms
    pub fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), EpdError<SPI::Error>> {
        self.rst.set_high().map_err(|_| EpdError::Pin)?;
        delay.delay_ms(20);

        self.rst.set_low().map_err(|_| EpdError::Pin)?;
        delay.delay_ms(10);

        self.rst.set_high().map_err(|_| EpdError::Pin)?;
        delay.delay_ms(20);

        Ok(())
    }

    /// Sends the startup/configuration commands needed to prepare the e-paper display
    pub fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), EpdError<SPI::Error>> {
        self.reset(delay)?;

        self.send_command(Command::SoftwareReset as u8)?;
        self.wait_until_idle(delay, 15_000)?;

        self.send_command(Command::DriverOutputControl as u8)?;
        self.send_data(&[0xB7, 0x00, 0x00])?;

        self.send_command(Command::DataEntryModeSetting as u8)?;
        self.send_data(&[0x03])?;

        self.set_full_ram_window()?;

        self.send_command(Command::BorderWaveformControl as u8)?;
        self.send_data(&[0x05])?;

        self.send_command(Command::TemperatureSensorControl as u8)?;
        self.send_data(&[0x80])?;

        self.send_command(Command::DisplayUpdateControl1 as u8)?;
        self.send_data(&[0x00, 0x80])?;

        self.set_ram_cursor(0, 0)?;

        Ok(())
    }

    /// Writes the full framebuffer to display RAM and triggers a full refresh
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
        self.send_command(Command::WriteRam as u8)?;
        self.send_data(buffer)?;

        self.refresh_full(delay)
    }

    /// Puts the display into deep sleep mode to save power. The display can be woken up by toggling the RST pin.
    pub fn sleep(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::DeepSleepMode as u8)?;
        self.send_data(&[0x01])?;
        Ok(())
    }

    /// Sends one command byte over SPI with DC low
    pub fn send_command(&mut self, command: u8) -> Result<(), EpdError<SPI::Error>> {
        self.dc.set_low().map_err(|_| EpdError::Pin)?;
        self.cs.set_low().map_err(|_| EpdError::Pin)?;

        self.spi.write(&[command]).map_err(EpdError::Spi)?;

        self.cs.set_high().map_err(|_| EpdError::Pin)?;
        Ok(())
    }

    /// Sends data bytes over SPI with DC high
    pub fn send_data(&mut self, data: &[u8]) -> Result<(), EpdError<SPI::Error>> {
        self.dc.set_high().map_err(|_| EpdError::Pin)?;
        self.cs.set_low().map_err(|_| EpdError::Pin)?;

        self.spi.write(data).map_err(EpdError::Spi)?;

        self.cs.set_high().map_err(|_| EpdError::Pin)?;
        Ok(())
    }

    /// Sends the update command that makes the display physically refresh
    fn refresh_full<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::DisplayUpdateControl2 as u8)?;
        self.send_data(&[0xF7])?;

        self.send_command(Command::MasterActivation as u8)?;

        self.wait_until_idle(delay, 15_000)
    }

    /// Waits until the BUSY pin says the display is ready, or returns timeout error if it takes too long
    fn wait_until_idle<D: DelayNs>(
        &mut self,
        delay: &mut D,
        timeout_ms: u32,
    ) -> Result<(), EpdError<SPI::Error>> {
        let mut elapsed = 0;

        while self.busy.is_high().map_err(|_| EpdError::Pin)? {
            if elapsed >= timeout_ms {
                return Err(EpdError::BusyTimeout);
            }

            delay.delay_ms(10);
            elapsed += 10;
        }

        Ok(())
    }

    /// Selects the whole display memory area for writing
    fn set_full_ram_window(&mut self) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::SetRamXAddressStartEnd as u8)?;
        self.send_data(&[0, (ROW_BYTES - 1) as u8])?;

        self.send_command(Command::SetRamYAddressStartEnd as u8)?;
        self.send_data(&[
            0,
            0,
            ((HEIGHT - 1) & 0xFF) as u8,
            ((HEIGHT - 1) >> 8) as u8,
        ])?;

        Ok(())
    }

    /// Sets the current RAM write position inside the display controller
    fn set_ram_cursor(&mut self, x_byte: u8, y: u16) -> Result<(), EpdError<SPI::Error>> {
        self.send_command(Command::SetRamXAddressCounter as u8)?;
        self.send_data(&[x_byte])?;

        self.send_command(Command::SetRamYAddressCounter as u8)?;
        self.send_data(&[(y & 0xFF) as u8, (y >> 8) as u8])?;

        Ok(())
    }
}