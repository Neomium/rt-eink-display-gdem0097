use embedded_hal::{
    delay::DelayNs,
    digital::{
        Error as DigitalError, ErrorKind as DigitalErrorKind, ErrorType as DigitalErrorType,
    },
    spi::{
        Error as SpiError, ErrorKind as SpiErrorKind, ErrorType as SpiErrorType,
        Operation as SpiOperation, SpiDevice,
    },
};
use mockall::{Sequence, mock, predicate::eq};

use std::{vec, vec::Vec};

use crate::driver::{RESET_LOW_TIME_NS, RESET_RECOVERY_TIME_NS};
use crate::{BUFFER_SIZE, Color, Config, Display, Error, FrameBuffer, HEIGHT, Region, WIDTH};

const SOFTWARE_RESET: u8 = 0x12;
const DRIVER_OUTPUT_CONTROL: u8 = 0x01;
const DATA_ENTRY_MODE_SETTING: u8 = 0x11;
const TEMPERATURE_SENSOR_CONTROL: u8 = 0x18;
const MASTER_ACTIVATION: u8 = 0x20;
const DISPLAY_UPDATE_CONTROL_1: u8 = 0x21;
const DISPLAY_UPDATE_CONTROL_2: u8 = 0x22;
const WRITE_RAM: u8 = 0x24;
const BORDER_WAVEFORM_CONTROL: u8 = 0x3C;
const SET_RAM_X_WINDOW: u8 = 0x44;
const SET_RAM_Y_WINDOW: u8 = 0x45;
const SET_RAM_X_CURSOR: u8 = 0x4E;
const SET_RAM_Y_CURSOR: u8 = 0x4F;
const DEEP_SLEEP: u8 = 0x10;
const TEST_BUSY_TIMEOUT_US: u32 = 1_000;
const TEST_BUSY_POLL_INTERVAL_US: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestSpiError {
    WriteFailed,
}

impl SpiError for TestSpiError {
    fn kind(&self) -> SpiErrorKind {
        SpiErrorKind::Other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestPinError {
    SetLow,
    Read,
    Write,
}

impl DigitalError for TestPinError {
    fn kind(&self) -> DigitalErrorKind {
        DigitalErrorKind::Other
    }
}

#[cfg(test)]
mock! {
    Spi {}

    impl SpiErrorType for Spi {
        type Error = TestSpiError;
    }

    impl SpiDevice<u8> for Spi {

        fn transaction<'a>(&mut self, operations: &mut [SpiOperation<'a, u8>]) -> Result<(), TestSpiError>;

    }
}

mock! {
    OutputPin {}

    impl DigitalErrorType for OutputPin {
        type Error = TestPinError;
    }

    impl embedded_hal::digital::OutputPin for OutputPin {
        fn set_low(&mut self) -> Result<(), TestPinError>;
        fn set_high(&mut self) -> Result<(), TestPinError>;
    }
}

mock! {
    InputPin {}

    impl DigitalErrorType for InputPin {
        type Error = TestPinError;
    }

    impl embedded_hal::digital::InputPin for InputPin {
        fn is_high(&mut self) -> Result<bool, TestPinError>;
        fn is_low(&mut self) -> Result<bool, TestPinError>;
    }
}

mock! {
    Delay {}

    impl DelayNs for Delay {
        fn delay_ns(&mut self, ns: u32);
    }
}

type MockDisplay = Display<MockSpi, MockOutputPin, MockOutputPin, MockInputPin, MockDelay>;

struct MockParts {
    spi: MockSpi,
    dc: MockOutputPin,
    rst: MockOutputPin,
    busy: MockInputPin,
    delay: MockDelay,
}

enum TransferKind {
    Command,
    Data,
}

fn mock_parts(writes: &[&[u8]], command_or_data: &[TransferKind]) -> MockParts {
    assert_eq!(writes.len(), command_or_data.len());

    let mut spi = MockSpi::new();
    let mut spi_sequence = Sequence::new();

    for write in writes {
        let expected = write.to_vec();
        spi.expect_transaction()
            .withf(move |operations| {
                matches!(
                    operations,
                    [SpiOperation::Write(data)]
                        if *data == expected.as_slice()
                )
            })
            .times(1)
            .in_sequence(&mut spi_sequence)
            .returning(|_| Ok(()));
    }

    let mut dc = MockOutputPin::new();
    let mut dc_sequence = Sequence::new();

    for transfer_kind in command_or_data {
        match transfer_kind {
            TransferKind::Command => {
                dc.expect_set_low()
                    .times(1)
                    .in_sequence(&mut dc_sequence)
                    .returning(|| Ok(()));
            }
            TransferKind::Data => {
                dc.expect_set_high()
                    .times(1)
                    .in_sequence(&mut dc_sequence)
                    .returning(|| Ok(()));
            }
        }
    }

    MockParts {
        spi,
        dc,
        rst: MockOutputPin::new(),
        busy: MockInputPin::new(),
        delay: MockDelay::new(),
    }
}

fn test_config() -> Config {
    Config {
        busy_timeout_us: TEST_BUSY_TIMEOUT_US,
        busy_poll_interval_us: TEST_BUSY_POLL_INTERVAL_US,
        wait_after_refresh: true,
    }
}

fn make_display(parts: MockParts) -> MockDisplay {
    let MockParts {
        spi,
        dc,
        rst,
        busy,
        delay,
    } = parts;
    Display::new(spi, dc, rst, busy, delay, test_config())
}

fn expect_reset_pin_sequence(rst: &mut MockOutputPin, delay: &mut MockDelay) {
    let mut reset_sequence = Sequence::new();

    rst.expect_set_low()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));

    rst.expect_set_high()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));

    let mut delay_sequence = Sequence::new();

    delay
        .expect_delay_ns()
        .with(eq(200_000))
        .times(1)
        .in_sequence(&mut delay_sequence)
        .return_const(());

    delay
        .expect_delay_ns()
        .with(eq(200_000))
        .times(1)
        .in_sequence(&mut delay_sequence)
        .return_const(());
}

fn expect_busy_idle(busy: &mut MockInputPin) {
    busy.expect_is_high().times(1).returning(|| Ok(false));
}

#[test]
fn release_returns_all_display_resources() {
    let display = make_display(mock_parts(&[], &[]));
    let (_spi, _dc, _rst, _busy, _delay) = display.release();
}

#[test]
fn reset_toggles_reset_pin_and_waits_until_idle() {
    let mut parts = mock_parts(&[], &[]);

    expect_reset_pin_sequence(&mut parts.rst, &mut parts.delay);
    expect_busy_idle(&mut parts.busy);

    let mut display = make_display(parts);

    display.reset().unwrap();
}

#[test]
fn reset_returns_pin_error_when_reset_pin_cannot_be_set_low() {
    let spi = MockSpi::new();
    let dc = MockOutputPin::new();

    let mut reset = MockOutputPin::new();
    reset
        .expect_set_low()
        .times(1)
        .returning(|| Err(TestPinError::Write));

    let busy = MockInputPin::new();
    let delay = MockDelay::new();

    let mut display = Display::new(spi, dc, reset, busy, delay, Config::default());

    assert_eq!(
        display.reset().unwrap_err(),
        Error::Pin(TestPinError::Write)
    );
}

#[test]
fn reset_returns_busy_timeout_when_busy_remains_high() {
    let spi = MockSpi::new();
    let dc = MockOutputPin::new();

    let mut reset = MockOutputPin::new();

    reset.expect_set_low().times(1).returning(|| Ok(()));

    reset.expect_set_high().times(1).returning(|| Ok(()));

    let mut busy = MockInputPin::new();

    busy.expect_is_high().returning(|| Ok(true));

    let config = Config {
        busy_timeout_us: 500,
        busy_poll_interval_us: 500,
        ..Config::default()
    };

    let mut delay = MockDelay::new();

    // reset(): delay after pulling RESET low
    delay
        .expect_delay_ns()
        .with(eq(RESET_LOW_TIME_NS))
        .times(1)
        .returning(|_| ());
    // reset(): delay after pulling RESET high
    delay
        .expect_delay_ns()
        .with(eq(RESET_RECOVERY_TIME_NS))
        .times(1)
        .returning(|_| ());

    // wait_until_idle(): BUSY polling delay
    delay
        .expect_delay_ns()
        .with(eq(config.busy_poll_interval_us * 1_000))
        .returning(|_| ());

    let mut display = Display::new(spi, dc, reset, busy, delay, config);

    assert_eq!(display.reset().unwrap_err(), Error::BusyTimeout);
}

#[test]
fn init_sends_the_complete_controller_configuration() {
    let writes: &[&[u8]] = &[
        &[SOFTWARE_RESET],
        &[DRIVER_OUTPUT_CONTROL],
        &[0xB7, 0x00, 0x00],
        &[DATA_ENTRY_MODE_SETTING],
        &[0x03],
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[BORDER_WAVEFORM_CONTROL],
        &[0x05],
        &[TEMPERATURE_SENSOR_CONTROL],
        &[0x80],
        &[DISPLAY_UPDATE_CONTROL_1],
        &[0x00, 0x80],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
    ];
    let mut parts = mock_parts(writes, &kinds);
    expect_reset_pin_sequence(&mut parts.rst, &mut parts.delay);
    parts.busy.expect_is_high().times(2).returning(|| Ok(false));
    let mut display = make_display(parts);

    display.init().unwrap();
}

#[test]
fn show_full_writes_the_entire_framebuffer_and_triggers_full_refresh() {
    let framebuffer = FrameBuffer::new();

    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
        framebuffer.as_bytes(),
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];
    let mut parts = mock_parts(&writes, &kinds);
    parts.busy.expect_is_high().times(1).returning(|| Ok(false));
    let mut display = make_display(parts);

    display.show_full(&framebuffer).unwrap();
}

#[test]
fn show_full_transfers_the_complete_nonuniform_framebuffer_unchanged() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(0, 0, Color::Black).unwrap();
    framebuffer.set_pixel(7, 0, Color::Black).unwrap();
    framebuffer.set_pixel(8, 0, Color::Black).unwrap();
    framebuffer.set_pixel(24, 17, Color::Black).unwrap();
    framebuffer.set_pixel(63, 92, Color::Black).unwrap();
    framebuffer
        .set_pixel(WIDTH - 1, HEIGHT - 1, Color::Black)
        .unwrap();

    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
        framebuffer.as_bytes(),
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];

    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let mut parts = mock_parts(&writes, &kinds);

    parts.busy.expect_is_high().times(1).returning(|| Ok(false));

    let mut display = make_display(parts);

    display.show_full(&framebuffer).unwrap();
}

#[test]
fn show_full_returns_spi_error_when_framebuffer_transfer_fails() {
    let framebuffer = FrameBuffer::filled(Color::Black);

    let successful_writes: &[&[u8]] = &[
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
    ];

    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let mut parts = mock_parts(successful_writes, &kinds);

    parts
        .spi
        .expect_transaction()
        .withf(|operations| {
            matches!(
                operations,
                [SpiOperation::Write(data)]
                    if *data == [0x00; BUFFER_SIZE]
            )
        })
        .times(1)
        .returning(|_| Err(TestSpiError::WriteFailed));

    parts.dc.expect_set_high().times(1).returning(|| Ok(()));

    let mut display = make_display(parts);

    let error = display.show_full(&framebuffer).unwrap_err();

    assert!(matches!(error, Error::Spi(TestSpiError::WriteFailed)));
}

#[test]
fn show_partial_writes_only_the_requested_rows_and_bytes() {
    let mut framebuffer = FrameBuffer::new();
    framebuffer.clear(Color::Black);
    let region = Region::new(8, 2, 16, 2).unwrap();

    let writes: &[&[u8]] = &[
        &[SET_RAM_X_WINDOW],
        &[0x01, 0x02],
        &[SET_RAM_Y_WINDOW],
        &[0x02, 0x00, 0x03, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x01],
        &[SET_RAM_Y_CURSOR],
        &[0x02, 0x00],
        &[WRITE_RAM],
        &[0x00, 0x00],
        &[0x00, 0x00],
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xFF],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let mut parts = mock_parts(writes, &kinds);
    parts.busy.expect_is_high().times(1).returning(|| Ok(false));

    let mut display = make_display(parts);

    display.show_partial(&framebuffer, region).unwrap();
}

#[test]
fn sleep_sends_the_deep_sleep_command_and_check_code() {
    let writes: &[&[u8]] = &[&[DEEP_SLEEP], &[0x01]];
    let kinds = [TransferKind::Command, TransferKind::Data];

    let mut display = make_display(mock_parts(writes, &kinds));

    display.sleep().unwrap();
}

#[test]
fn show_full_busy_pin_is_polled_until_the_display_becomes_idle() {
    let framebuffer = FrameBuffer::new();
    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
        framebuffer.as_bytes(),
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let mut parts = mock_parts(&writes, &kinds);

    let mut busy_sequence = Sequence::new();
    parts
        .busy
        .expect_is_high()
        .times(1)
        .in_sequence(&mut busy_sequence)
        .returning(|| Ok(true));
    parts
        .busy
        .expect_is_high()
        .times(1)
        .in_sequence(&mut busy_sequence)
        .returning(|| Ok(false));
    parts
        .delay
        .expect_delay_ns()
        .with(eq(100_000))
        .times(1)
        .return_const(());

    let mut display = make_display(parts);
    display.show_full(&framebuffer).unwrap();
}

#[test]
fn show_full_returns_busy_timeout_when_busy_remains_high() {
    let framebuffer = FrameBuffer::new();
    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
        framebuffer.as_bytes(),
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let mut parts = mock_parts(&writes, &kinds);

    parts.busy.expect_is_high().returning(|| Ok(true));
    parts.delay.expect_delay_ns().return_const(());

    let mut display = make_display(parts);
    let error = display.show_full(&framebuffer).unwrap_err();

    assert!(matches!(error, Error::BusyTimeout));
}

#[test]
fn show_full_returns_pin_error_when_busy_pin_read_fails() {
    let framebuffer = FrameBuffer::new();

    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
        framebuffer.as_bytes(),
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];

    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let mut parts = mock_parts(&writes, &kinds);

    parts
        .busy
        .expect_is_high()
        .times(1)
        .returning(|| Err(TestPinError::Read));

    let mut display = make_display(parts);

    let error = display.show_full(&framebuffer).unwrap_err();

    assert!(matches!(error, Error::Pin(TestPinError::Read)));
}

#[test]
fn sleep_returns_spi_error_when_command_transfer_fails() {
    let mut spi = MockSpi::new();

    spi.expect_transaction()
        .withf(|operations| {
            matches!(
                operations,
                [SpiOperation::Write(data)] if *data == [DEEP_SLEEP]
            )
        })
        .times(1)
        .returning(|_| Err(TestSpiError::WriteFailed));

    let mut dc = MockOutputPin::new();

    dc.expect_set_low().times(1).returning(|| Ok(()));

    let mut display = Display::new(
        spi,
        dc,
        MockOutputPin::new(),
        MockInputPin::new(),
        MockDelay::new(),
        test_config(),
    );

    let error = display.sleep().unwrap_err();

    assert!(matches!(error, Error::Spi(TestSpiError::WriteFailed)));
}

#[test]
fn show_partial_reads_bytes_from_the_correct_framebuffer_offsets() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(8, 2, Color::Black).unwrap();
    framebuffer.set_pixel(16, 2, Color::Black).unwrap();
    framebuffer.set_pixel(9, 3, Color::Black).unwrap();
    framebuffer.set_pixel(18, 3, Color::Black).unwrap();

    let region = Region::new(8, 2, 16, 2).unwrap();
    let writes: &[&[u8]] = &[
        &[SET_RAM_X_WINDOW],
        &[0x01, 0x02],
        &[SET_RAM_Y_WINDOW],
        &[0x02, 0x00, 0x03, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x01],
        &[SET_RAM_Y_CURSOR],
        &[0x02, 0x00],
        &[WRITE_RAM],
        &[0x7F, 0x7F],
        &[0xBF, 0xDF],
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xFF],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];
    let mut parts = mock_parts(writes, &kinds);
    parts.busy.expect_is_high().times(1).returning(|| Ok(false));
    let mut display = make_display(parts);

    display.show_partial(&framebuffer, region).unwrap();
}

#[test]
fn init_returns_spi_error_when_software_reset_transfer_fails() {
    let mut parts = mock_parts(&[], &[]);

    expect_reset_pin_sequence(&mut parts.rst, &mut parts.delay);

    parts.busy.expect_is_high().times(1).returning(|| Ok(false));

    parts.dc.expect_set_low().times(1).returning(|| Ok(()));

    parts
        .spi
        .expect_transaction()
        .withf(|operations| {
            matches!(
                operations,
                [SpiOperation::Write(data)] if *data == [SOFTWARE_RESET]
            )
        })
        .times(1)
        .returning(|_| Err(TestSpiError::WriteFailed));

    let mut display = make_display(parts);

    let error = display.init().unwrap_err();

    assert!(matches!(error, Error::Spi(TestSpiError::WriteFailed)));
}

#[test]
fn init_returns_pin_error_when_reset_pin_cannot_be_set_low() {
    let mut parts = mock_parts(&[], &[]);

    parts
        .rst
        .expect_set_low()
        .times(1)
        .returning(|| Err(TestPinError::SetLow));

    let mut display = make_display(parts);

    let error = display.init().unwrap_err();

    assert!(matches!(error, Error::Pin(TestPinError::SetLow)));
}

#[test]
fn init_returns_busy_timeout_when_display_remains_busy() {
    let mut parts = mock_parts(&[], &[]);

    let mut reset_sequence = Sequence::new();

    parts
        .rst
        .expect_set_low()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));

    parts
        .rst
        .expect_set_high()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));

    let mut delay_sequence = Sequence::new();

    parts
        .delay
        .expect_delay_ns()
        .with(eq(200_000))
        .times(1)
        .in_sequence(&mut delay_sequence)
        .return_const(());

    parts
        .delay
        .expect_delay_ns()
        .with(eq(200_000))
        .times(1)
        .in_sequence(&mut delay_sequence)
        .return_const(());

    parts.busy.expect_is_high().returning(|| Ok(true));

    parts.delay.expect_delay_ns().return_const(());

    let mut display = make_display(parts);

    let error = display.init().unwrap_err();

    assert!(matches!(error, Error::BusyTimeout));
}

#[test]
fn show_full_does_not_wait_when_refresh_waiting_is_disabled() {
    let framebuffer = FrameBuffer::new();
    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x00, 0x0A],
        &[SET_RAM_Y_WINDOW],
        &[0x00, 0x00, 0xB7, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x00],
        &[SET_RAM_Y_CURSOR],
        &[0x00, 0x00],
        &[WRITE_RAM],
        framebuffer.as_bytes(),
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];
    let parts = mock_parts(&writes, &kinds);
    let MockParts {
        spi,
        dc,
        rst,
        busy,
        delay,
    } = parts;
    let config = Config {
        wait_after_refresh: false,
        ..test_config()
    };
    let mut display = Display::new(spi, dc, rst, busy, delay, config);

    display.show_full(&framebuffer).unwrap();
}

#[test]
fn show_partial_does_not_wait_when_refresh_waiting_is_disabled() {
    let framebuffer = FrameBuffer::new();
    let region = Region::new(8, 2, 16, 2).unwrap();

    let writes: Vec<&[u8]> = vec![
        &[SET_RAM_X_WINDOW],
        &[0x01, 0x02],
        &[SET_RAM_Y_WINDOW],
        &[0x02, 0x00, 0x03, 0x00],
        &[SET_RAM_X_CURSOR],
        &[0x01],
        &[SET_RAM_Y_CURSOR],
        &[0x02, 0x00],
        &[WRITE_RAM],
        &framebuffer.as_bytes()[23..25],
        &framebuffer.as_bytes()[34..36],
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xFF],
        &[MASTER_ACTIVATION],
    ];

    let kinds = [
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Data,
        TransferKind::Command,
        TransferKind::Data,
        TransferKind::Command,
    ];

    let parts = mock_parts(&writes, &kinds);

    let MockParts {
        spi,
        dc,
        rst,
        busy,
        delay,
    } = parts;

    let config = Config {
        wait_after_refresh: false,
        ..test_config()
    };

    let mut display = Display::new(spi, dc, rst, busy, delay, config);

    display.show_partial(&framebuffer, region).unwrap();
}
