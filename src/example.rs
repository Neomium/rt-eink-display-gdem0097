use core::convert::Infallible;

use embedded_hal::{
    delay::DelayNs,
    digital::{ErrorType as DigitalErrorType, InputPin, OutputPin},
    spi::{ErrorType as SpiErrorType, Operation, SpiDevice},
};

#[derive(Default)]
pub struct ExampleSpiDevice;

impl SpiErrorType for ExampleSpiDevice {
    type Error = Infallible;
}

impl SpiDevice<u8> for ExampleSpiDevice {
    fn transaction(
        &mut self,
        _operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct ExampleOutputPin;

impl DigitalErrorType for ExampleOutputPin {
    type Error = Infallible;
}

impl OutputPin for ExampleOutputPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct ExampleInputPin;

impl DigitalErrorType for ExampleInputPin {
    type Error = Infallible;
}

impl InputPin for ExampleInputPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[derive(Default)]
pub struct ExampleDelay;

impl DelayNs for ExampleDelay {
    fn delay_ns(&mut self, _ns: u32) {}
}