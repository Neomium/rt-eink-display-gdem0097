//! Timing regression: BUSY starts low, asserts late, then clears.
use crate::{Config, Display, Error};
use core::convert::Infallible;
use embedded_hal::{
    digital::{ErrorType as PinError, InputPin, OutputPin},
    spi::{ErrorType, Operation},
};
use std::{cell::Cell, rc::Rc};

#[cfg(feature = "async")]
macro_rules! call {
    ($e:expr) => {
        $e.await
    };
}
#[cfg(not(feature = "async"))]
macro_rules! call {
    ($e:expr) => {
        $e
    };
}
macro_rules! test_both {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() {
            #[cfg(feature = "async")]
            {
                use core::{future::Future, task::{Context, Poll, Waker}};
                let mut f = core::pin::pin!(async $body);
                let mut cx = Context::from_waker(Waker::noop());
                assert!(matches!(f.as_mut().poll(&mut cx), Poll::Ready(())));
            }
            #[cfg(not(feature = "async"))]
            $body
        }
    };
}

#[derive(Clone, Default)]
struct Clock(Rc<Cell<u32>>);
impl embedded_hal::delay::DelayNs for Clock {
    fn delay_ns(&mut self, ns: u32) {
        self.0.set(self.0.get() + ns / 1000);
    }
}
impl embedded_hal_async::delay::DelayNs for Clock {
    async fn delay_ns(&mut self, ns: u32) {
        self.0.set(self.0.get() + ns / 1000);
    }
}
struct Busy {
    clock: Clock,
    stuck: bool,
}
impl PinError for Busy {
    type Error = Infallible;
}
impl InputPin for Busy {
    fn is_high(&mut self) -> Result<bool, Infallible> {
        let us = self.clock.0.get();
        Ok(us >= 5_000 && (self.stuck || us < 30_000))
    }
    fn is_low(&mut self) -> Result<bool, Infallible> {
        self.is_high().map(|b| !b)
    }
}
struct Pin;
impl PinError for Pin {
    type Error = Infallible;
}
impl OutputPin for Pin {
    fn set_low(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
}
struct Spi;
impl ErrorType for Spi {
    type Error = Infallible;
}
impl embedded_hal::spi::SpiDevice for Spi {
    fn transaction(&mut self, _: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        Ok(())
    }
}
impl embedded_hal_async::spi::SpiDevice for Spi {
    async fn transaction(&mut self, _: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        Ok(())
    }
}

test_both!(refresh_waits_for_delayed_busy_in_both_waveforms, {
    for partial in [false, true] {
        let clock = Clock::default();
        let busy = Busy {
            clock: clock.clone(),
            stuck: false,
        };
        let mut d = Display::new(Spi, Pin, Pin, busy, clock.clone(), Config::default());
        if partial {
            call!(d.refresh_partial()).unwrap();
        } else {
            call!(d.refresh_full()).unwrap();
        }
        assert!(clock.0.get() >= 30_000);
    }
});

test_both!(manual_wait_also_observes_delayed_busy, {
    let clock = Clock::default();
    let busy = Busy {
        clock: clock.clone(),
        stuck: false,
    };
    let config = Config {
        wait_after_refresh: false,
        ..Config::default()
    };
    let mut d = Display::new(Spi, Pin, Pin, busy, clock.clone(), config);
    call!(d.refresh_partial()).unwrap();
    assert_eq!(clock.0.get(), 0);
    call!(d.wait_until_idle()).unwrap();
    assert!(clock.0.get() >= 30_000);
    let elapsed = clock.0.get();
    call!(d.wait_until_idle()).unwrap();
    assert_eq!(clock.0.get(), elapsed);
});

test_both!(delayed_busy_still_times_out, {
    let clock = Clock::default();
    let busy = Busy {
        clock: clock.clone(),
        stuck: true,
    };
    let config = Config {
        busy_timeout_us: 40_000,
        ..Config::default()
    };
    let mut d = Display::new(Spi, Pin, Pin, busy, clock, config);
    assert_eq!(call!(d.refresh_partial()), Err(Error::BusyTimeout));
});
