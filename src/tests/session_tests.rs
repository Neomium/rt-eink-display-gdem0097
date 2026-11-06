use crate::{Color, Config, Display, FrameBuffer, HEIGHT, Region, WIDTH};
use core::convert::Infallible;

#[cfg(feature = "async")]
use core::{
    future::Future,
    task::{Context, Poll, Waker},
};

use embedded_hal::{
    digital::{ErrorType as PinError, InputPin, OutputPin},
    spi::{ErrorType, Operation},
};

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    vec::Vec,
};

// Shared pin state lets the SPI mock distinguish commands from data.
#[derive(Clone)]
struct Pin(Rc<Cell<bool>>);

impl PinError for Pin {
    type Error = Infallible;
}

impl OutputPin for Pin {
    fn set_low(&mut self) -> Result<(), Infallible> {
        self.0.set(false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Infallible> {
        self.0.set(true);
        Ok(())
    }
}

impl InputPin for Pin {
    fn is_high(&mut self) -> Result<bool, Infallible> {
        Ok(self.0.get())
    }

    fn is_low(&mut self) -> Result<bool, Infallible> {
        Ok(!self.0.get())
    }
}

// Each entry records the DC level (false = command, true = data) and payload.
type SpiTrace = Rc<RefCell<Vec<(bool, Vec<u8>)>>>;

struct Spi {
    dc: Pin,
    trace: SpiTrace,
}

impl ErrorType for Spi {
    type Error = Infallible;
}

impl Spi {
    fn record(&mut self, ops: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        for op in ops {
            if let Operation::Write(data) = op {
                self.trace
                    .borrow_mut()
                    .push((self.dc.0.get(), data.to_vec()));
            }
        }
        Ok(())
    }
}

impl embedded_hal::spi::SpiDevice for Spi {
    fn transaction(&mut self, ops: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        self.record(ops)
    }
}

impl embedded_hal_async::spi::SpiDevice for Spi {
    async fn transaction(&mut self, ops: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        self.record(ops)
    }
}

// Keep both driver variants independent of real time.
struct Delay;

impl embedded_hal::delay::DelayNs for Delay {
    fn delay_ns(&mut self, _: u32) {}
}

impl embedded_hal_async::delay::DelayNs for Delay {
    async fn delay_ns(&mut self, _: u32) {}
}

// Poll the immediately ready mocks without an async runtime.
#[cfg(feature = "async")]
fn run(f: impl Future<Output = ()>) {
    let mut f = core::pin::pin!(f);
    let mut cx = Context::from_waker(Waker::noop());

    for _ in 0..1000 {
        if let Poll::Ready(()) = f.as_mut().poll(&mut cx) {
            return;
        }
    }

    panic!("unexpected pending");
}

// Changed bounds must cover erased pixels as well as newly drawn pixels.
#[test]
fn changed_bounds_are_byte_aligned_and_cover_both_erased_and_added_pixels() {
    let mut old = FrameBuffer::new();
    old.set_pixel(1, 4, Color::Black).unwrap();

    let mut new = FrameBuffer::new();
    new.set_pixel(17, 8, Color::Black).unwrap();

    assert_eq!(
        new.changed_region(&old),
        Some(Region::new(0, 4, 24, 5).unwrap())
    );
    assert_eq!(old.changed_region(&old), None);
}

// Verify the blocking session command sequence.
#[cfg(not(feature = "async"))]
#[test]
fn mode_preparation_reinitializes_before_ram_writes() {
    {
        // Create a display with a shared SPI trace.
        let pin = || Pin(Rc::new(Cell::new(false)));
        let dc = pin();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let spi = Spi {
            dc: dc.clone(),
            trace: trace.clone(),
        };
        let mut d = Display::new(spi, dc, pin(), pin(), Delay, Config::default());
        let frame = FrameBuffer::new();

        d.init().unwrap();
        trace.borrow_mut().clear();

        // Seed both RAM banks without triggering a refresh.
        d.write_base_frame(&frame).unwrap();
        let commands = || {
            trace
                .borrow()
                .iter()
                .filter(|(data, _)| !data)
                .map(|(_, v)| v[0])
                .collect::<Vec<_>>()
        };

        assert!(commands().contains(&0x24));
        assert!(commands().contains(&0x26));
        assert!(!commands().contains(&0x20));

        // Establish the displayed baseline, then reinitialize partial mode.
        d.refresh_full().unwrap();
        trace.borrow_mut().clear();

        d.prepare_partial().unwrap();
        d.write_previous_region(&frame, Region::new(0, 0, WIDTH, HEIGHT).unwrap())
            .unwrap();
        d.write_frame(&frame).unwrap();

        assert_eq!(commands().iter().filter(|&&c| c == 0x24).count(), 1);
        assert_eq!(commands().iter().filter(|&&c| c == 0x26).count(), 1);
        let cmds = commands();
        let position = |cmd| cmds.iter().position(|&c| c == cmd).unwrap();
        assert!(position(0x12) < position(0x26));
        assert!(position(0x26) < position(0x24));
        assert!(
            trace
                .borrow()
                .windows(2)
                .any(|w| w[0] == (false, std::vec![0x3C]) && w[1] == (true, std::vec![0x80]))
        );
        assert!(!commands().contains(&0x20));

        // The mode preparation resets configuration before writes; activation does not reset.
        d.refresh_partial().unwrap();
        d.wait_until_idle().unwrap();
        assert_eq!(commands().iter().filter(|&&c| c == 0x20).count(), 1);
        assert_eq!(commands().iter().filter(|&&c| c == 0x12).count(), 1);
    }
}

// Verify the same session behavior with the async driver.
#[cfg(feature = "async")]
#[test]
fn mode_preparation_reinitializes_before_ram_writes() {
    run(async {
        // Create a display with a shared SPI trace.
        let pin = || Pin(Rc::new(Cell::new(false)));
        let dc = pin();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let spi = Spi {
            dc: dc.clone(),
            trace: trace.clone(),
        };
        let mut d = Display::new(spi, dc, pin(), pin(), Delay, Config::default());
        let frame = FrameBuffer::new();

        d.init().await.unwrap();
        trace.borrow_mut().clear();

        // Seed both RAM banks without triggering a refresh.
        d.write_base_frame(&frame).await.unwrap();
        let commands = || {
            trace
                .borrow()
                .iter()
                .filter(|(data, _)| !data)
                .map(|(_, v)| v[0])
                .collect::<Vec<_>>()
        };

        assert!(commands().contains(&0x24));
        assert!(commands().contains(&0x26));
        assert!(!commands().contains(&0x20));

        // Establish the displayed baseline, then reinitialize partial mode.
        d.refresh_full().await.unwrap();
        trace.borrow_mut().clear();

        d.prepare_partial().await.unwrap();
        d.write_previous_region(&frame, Region::new(0, 0, WIDTH, HEIGHT).unwrap())
            .await
            .unwrap();
        d.write_frame(&frame).await.unwrap();

        assert_eq!(commands().iter().filter(|&&c| c == 0x24).count(), 1);
        assert_eq!(commands().iter().filter(|&&c| c == 0x26).count(), 1);
        let cmds = commands();
        let position = |cmd| cmds.iter().position(|&c| c == cmd).unwrap();
        assert!(position(0x12) < position(0x26));
        assert!(position(0x26) < position(0x24));
        assert!(
            trace
                .borrow()
                .windows(2)
                .any(|w| w[0] == (false, std::vec![0x3C]) && w[1] == (true, std::vec![0x80]))
        );
        assert!(!commands().contains(&0x20));

        // The mode preparation resets configuration before writes; activation does not reset.
        d.refresh_partial().await.unwrap();
        d.wait_until_idle().await.unwrap();
        assert_eq!(commands().iter().filter(|&&c| c == 0x20).count(), 1);
        assert_eq!(commands().iter().filter(|&&c| c == 0x12).count(), 1);
    });
}
