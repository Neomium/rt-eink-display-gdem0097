use core::convert::Infallible;

use embedded_hal::{
    delay::DelayNs,
    digital::ErrorType as DigitalErrorType,
    spi::{ErrorType as SpiErrorType, SpiBus},
};
use mockall::{mock, predicate::eq, Sequence};

use std::{vec, vec::Vec};

use crate::{BUFFER_SIZE, Color, EpdDisplay, EpdError, FrameBuffer, Region};

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


#[cfg(test)]
mock! {
    Spi {}

    impl SpiErrorType for Spi {
        type Error = Infallible;
    }

    impl SpiBus<u8> for Spi {
        fn read(&mut self, words: &mut [u8]) -> Result<(), Infallible>;
        fn write(&mut self, words: &[u8]) -> Result<(), Infallible>;
        fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Infallible>;
        fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Infallible>;
        fn flush(&mut self) -> Result<(), Infallible>;
    }
}

mock! {
    OutputPin {}

    impl DigitalErrorType for OutputPin {
        type Error = Infallible;
    }

    impl embedded_hal::digital::OutputPin for OutputPin {
        fn set_low(&mut self) -> Result<(), Infallible>;
        fn set_high(&mut self) -> Result<(), Infallible>;
    }
}

mock! {
    InputPin {}

    impl DigitalErrorType for InputPin {
        type Error = Infallible;
    }

    impl embedded_hal::digital::InputPin for InputPin {
        fn is_high(&mut self) -> Result<bool, Infallible>;
        fn is_low(&mut self) -> Result<bool, Infallible>;
    }
}

mock! {
    Delay {}

    impl DelayNs for Delay {
        fn delay_ns(&mut self, ns: u32);
    }
}

type MockDisplay =
    EpdDisplay<MockSpi, MockOutputPin, MockOutputPin, MockOutputPin, MockInputPin, MockDelay>;

struct MockParts {
    spi: MockSpi,
    cs: MockOutputPin,
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
        spi.expect_write()
            .withf(move |actual| actual == expected.as_slice())
            .times(1)
            .in_sequence(&mut spi_sequence)
            .returning(|_| Ok(()));
    }

    let mut cs = MockOutputPin::new();
    let mut cs_sequence = Sequence::new();
    cs.expect_set_high()
        .times(1)
        .in_sequence(&mut cs_sequence)
        .returning(|| Ok(()));
    for _ in writes {
        cs.expect_set_low()
            .times(1)
            .in_sequence(&mut cs_sequence)
            .returning(|| Ok(()));
        cs.expect_set_high()
            .times(1)
            .in_sequence(&mut cs_sequence)
            .returning(|| Ok(()));
    }

    let mut dc = MockOutputPin::new();
    let mut dc_sequence = Sequence::new();
    for transfer_kind in command_or_data {     
        match *transfer_kind {
            TransferKind::Data => {
                dc.expect_set_high()
                    .times(1)
                    .in_sequence(&mut dc_sequence)
                    .returning(|| Ok(()));
            }
            TransferKind::Command => {
                dc.expect_set_low()
                    .times(1)
                    .in_sequence(&mut dc_sequence)
                    .returning(|| Ok(()));
            }
        }
    }

    MockParts {
        spi,
        cs,
        dc,
        rst: MockOutputPin::new(),
        busy: MockInputPin::new(),
        delay: MockDelay::new(),
    }
        
}

fn make_display(parts: MockParts) -> MockDisplay {
    let MockParts {spi, cs, dc, rst, busy, delay} = parts;
    EpdDisplay::new(spi, cs, dc, rst, busy, delay)
        .expect("display construction should succeed")
}

fn expect_reset(rst: &mut MockOutputPin, delay: &mut MockDelay) {
    let mut reset_sequence = Sequence::new();
    rst.expect_set_high()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));
    rst.expect_set_low()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));
    rst.expect_set_high()
        .times(1)
        .in_sequence(&mut reset_sequence)
        .returning(|| Ok(()));

    let mut delay_sequence = Sequence::new();
    for ns in [20_000_000, 10_000_000, 20_000_000] {
        delay
            .expect_delay_ns()
            .with(eq(ns))
            .times(1)
            .in_sequence(&mut delay_sequence)
            .return_const(());
    }
}

#[test]
fn new_sets_chip_select_high_and_release_returns_all_resources() {
    let display = make_display(mock_parts(&[], &[]));
    let (_spi, _cs, _dc, _rst, _busy, _delay) = display.release();
}

#[test]
fn reset_toggles_reset_pin_and_waits_for_the_required_intervals() {
    let mut parts = mock_parts(&[], &[]);
    expect_reset(&mut parts.rst, &mut parts.delay);
    let mut display = make_display(parts);

    display.reset().expect("reset should succeed");
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
        TransferKind::Command, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command,
        TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data,
    ];
    let mut parts = mock_parts(writes, &kinds);
    expect_reset(&mut parts.rst, &mut parts.delay);
    parts.busy.expect_is_high().times(1).returning(|| Ok(false));
    let mut display = make_display(parts);

    display.init().expect("initialization should succeed");
}

#[test]
fn show_full_writes_the_entire_framebuffer_and_triggers_full_refresh() {
    let framebuffer = [0xA5; BUFFER_SIZE];
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
        &framebuffer,
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data,
        TransferKind::Command,
    ];
    let mut parts = mock_parts(&writes, &kinds);
    parts.busy.expect_is_high().times(1).returning(|| Ok(false));
    let mut display = make_display(parts);

    display
        .show_full(&framebuffer)
        .expect("full refresh should succeed");
}

#[test]
fn show_partial_writes_only_the_requested_rows_and_bytes() {
    let mut framebuffer = FrameBuffer::new();
    framebuffer.clear(Color::Black);
    let region = Region::new(8, 2, 16, 2);

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
        TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Data, TransferKind::Command,
        TransferKind::Data, TransferKind::Command,
    ];
    let mut parts = mock_parts(writes, &kinds);
    parts.busy.expect_is_high().times(1).returning(|| Ok(false));
    let mut display = make_display(parts); 

    display
        .show_partial(&framebuffer, region)
        .expect("partial refresh should succeed");
}

#[test]
fn sleep_sends_the_deep_sleep_command_and_check_code() {
    let writes: &[&[u8]] = &[&[DEEP_SLEEP], &[0x01]];
    let kinds = [TransferKind::Command, TransferKind::Data];
    let mut display = make_display(mock_parts(writes, &kinds));

    display.sleep().expect("sleep should succeed");
}

#[test]
fn zero_sized_partial_regions_are_rejected_without_hardware_access() {
    for region in [Region::new(0, 0, 0, 8), Region::new(0, 0, 8, 0)] {
        let mut display = make_display(mock_parts(&[], &[]));
        let error = display
            .show_partial(&FrameBuffer::new(), region)
            .expect_err("zero-sized region must fail");

        assert!(matches!(error, EpdError::InvalidRegion));
    }
}

#[test]
fn out_of_bounds_and_overflowing_partial_regions_are_rejected() {
    let regions = [
        Region::new(88, 0, 8, 8),
        Region::new(0, 184, 8, 8),
        Region::new(usize::MAX, 0, 8, 8),
        Region::new(0, usize::MAX, 8, 8),
    ];

    for region in regions {
        let mut display = make_display(mock_parts(&[], &[]));
        let error = display
            .show_partial(&FrameBuffer::new(), region)
            .expect_err("out-of-bounds region must fail");

        assert!(matches!(error, EpdError::InvalidRegion));
    }
}

#[test]
fn non_byte_aligned_partial_regions_are_rejected() {
    for region in [Region::new(1, 0, 8, 8), Region::new(0, 0, 7, 8)] {
        let mut display = make_display(mock_parts(&[], &[]));
        let error = display
            .show_partial(&FrameBuffer::new(), region)
            .expect_err("unaligned region must fail");

        assert!(matches!(error, EpdError::UnalignedRegion));
    }
}

#[test]
fn busy_pin_is_polled_until_the_display_becomes_idle() {
    let framebuffer = [0xFF; BUFFER_SIZE];
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
        &framebuffer,
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data,
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
        .with(eq(10_000_000))
        .times(1)
        .return_const(());

    let mut display = make_display(parts);
    display
        .show_full(&framebuffer)
        .expect("display should become idle after one poll delay");
}

#[test]
fn busy_remaining_high_returns_busy_timeout() {
    let framebuffer = [0xFF; BUFFER_SIZE];
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
        &framebuffer,
        &[DISPLAY_UPDATE_CONTROL_2],
        &[0xF7],
        &[MASTER_ACTIVATION],
    ];
    let kinds = [
        TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command, TransferKind::Data, TransferKind::Command,
    ];
    let mut parts = mock_parts(&writes, &kinds);

    parts.busy.expect_is_high().returning(|| Ok(true));
    parts.delay.expect_delay_ns().return_const(());

    let mut display = make_display(parts);
    let error = display
        .show_full(&framebuffer)
        .expect_err("display that remains busy must time out");

    assert!(matches!(error, EpdError::BusyTimeout));
}
