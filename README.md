# epd-gdem0097t61

[![CI](https://github.com/Neomium/rt-eink-display-gdem0097/actions/workflows/ci.yml/badge.svg)](https://github.com/Neomium/rt-eink-display-gdem0097/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/epd-gdem0097t61.svg)](https://crates.io/crates/epd-gdem0097t61)
[![Documentation](https://docs.rs/epd-gdem0097t61/badge.svg)](https://docs.rs/epd-gdem0097t61)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/epd-gdem0097t61.svg)](https://crates.io/crates/epd-gdem0097t61)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

A `no_std` Rust driver for the GDEM0097T61 monochrome e-paper display, built on `embedded-hal` 1.0 traits.

The driver is platform-independent and can be used with any microcontroller platform that provides compatible SPI, GPIO, and delay implementations.

## Features

- `no_std`
- `embedded-hal` 1.0
- `SpiDevice` support for shared SPI buses
- Full and partial display refresh
- Packed 1-bit framebuffer
- Configurable BUSY timeout and polling interval
- Configurable blocking or non-blocking refresh behavior
- Deep sleep support

## Supported hardware

| Display | Resolution | Colors |
|---|---:|---|
| GDEM0097T61 | 88 × 184 | Black / White |

## Installation

```toml
[dependencies]
epd-gdem0097t61 = "0.1"
```

## Configuration

`Config::default()` uses a 15-second BUSY timeout, a 500-microsecond polling interval, and waits for refresh operations to finish.

```rust
use epd_gdem0097t61::Config;

let config = Config {
    busy_timeout_us: 15_000_000,
    busy_poll_interval_us: 500,
    wait_after_refresh: true,
};
```

Set `wait_after_refresh` to `false` when the application should continue immediately after the refresh command is triggered. Reset and initialization still wait for the display controller because subsequent commands require it to be ready.

## Full refresh

Create the driver using platform-specific implementations of `SpiDevice`, `OutputPin`, `InputPin`, and `DelayNs`.

```rust,ignore
use epd_gdem0097t61::{Color, Config, Display, FrameBuffer};

let mut display = Display::new(
    spi,
    pin_dc,
    pin_reset,
    pin_busy,
    delay,
    Config::default(),
);

display.init()?;

let mut framebuffer = FrameBuffer::new();
framebuffer.clear(Color::White);
framebuffer.set_pixel(10, 20, Color::Black)?;

display.show_full(&framebuffer)?;
display.sleep()?;
```

Chip-select handling is performed by the `SpiDevice` implementation, so the display driver does not manage a CS pin directly.

## Partial refresh

A partial refresh updates a validated rectangular region.

```rust,ignore
use epd_gdem0097t61::{Color, FrameBuffer, Region};

let mut framebuffer = FrameBuffer::new();
framebuffer.set_pixel(16, 20, Color::Black)?;

let region = Region::new(
    16, // x
    16, // y
    32, // width
    24, // height
)?;

display.show_partial(&framebuffer, region)?;
```

Coordinates use the normal framebuffer coordinate system: `(0, 0)` is the top-left pixel, X increases to the right, and Y increases downward.

Because each byte stores eight horizontal pixels, `x` and `width` must be divisible by eight. `Region::new()` also rejects empty, overflowing, and out-of-bounds regions.

## Framebuffer

The display uses a packed 1-bit framebuffer:

- Black pixels are cleared bits (`0`).
- White pixels are set bits (`1`).
- Each byte stores eight horizontal pixels.
- The most significant bit represents the leftmost pixel in a byte.

The framebuffer dimensions are:

- Width: 88 pixels
- Height: 184 pixels
- Bytes per row: 11
- Buffer size: 2024 bytes

```rust
use epd_gdem0097t61::{Color, FrameBuffer};

let mut framebuffer = FrameBuffer::filled(Color::White);
framebuffer.set_pixel(20, 30, Color::Black).unwrap();

assert_eq!(
    framebuffer.color_of_pixel(20, 30),
    Ok(Color::Black)
);
```

## Public API

The crate exposes `Display`, `Config`, `FrameBuffer`, `Color`, `Region`, `Error`, `RegionError`, `FrameBufferError`, and the display-size constants.

## Releases

CI runs formatting, Clippy, tests, documentation, feature checks, and an MSRV check on every push and pull request.

Publishing uses crates.io Trusted Publishing, so no long-lived crates.io token is stored in GitHub. Before the first automated release:

1. Create a GitHub environment named `release` and add any desired deployment protection rules.
2. On crates.io, configure this repository as a trusted publisher for the `release.yml` workflow and the `release` environment.
3. Update the package version in `Cargo.toml`, then push a matching tag such as `v0.2.0`.

A valid version tag publishes the crate to crates.io and creates a GitHub release containing the packaged `.crate` archive.


## Persistent partial-update sessions

The low-level API separates RAM transfers from panel refresh. With the `async`
feature, await each operation (the synchronous API has the same names):

1. Initialize with `init()`.
2. `prepare_full()`, `write_base_frame(frame)`, then `refresh_full()` establish
   the initial image. After completion call `write_base_frame(frame)` again to
   synchronize both RAM banks explicitly.
3. For a partial update, call `prepare_partial()`. This performs hardware and
   software reset and restores controller configuration before selecting the
   partial border waveform. RAM retention must not be assumed.
4. Write the **entire last confirmed image** with `write_previous_region(old,
   Region::new(0, 0, WIDTH, HEIGHT).unwrap())`, then the complete desired image
   with `write_frame(new)`. Call `refresh_partial()` once.
5. After completion, synchronize **both complete banks** using
   `write_base_frame(new)`, then commit `new` as the application's confirmed image.

Both preparation methods reset/reinitialize the controller. Refill both banks
before activation, even when only a small region changed. This conservative
sequence retains the partial waveform while increasing SPI traffic.

The default configuration waits for refresh completion. After master activation,
`wait_until_idle()` first allows 10 ms for BUSY to assert, independently of the
polling interval, then polls until idle with a timeout. If `wait_after_refresh`
is false, call `wait_until_idle()` explicitly before synchronizing RAM or issuing
another operation. This guard also applies to that deferred wait.

`show_full()` and `show_partial()` remain low-level convenience methods; they do
not manage previous-image RAM or mode preparation. For alternating full/partial
updates use the sequence above. After sleep or any I/O error, treat the baseline
as invalid and rebuild it with a full refresh. Do not commit the desired image
until refresh and both synchronization transfers have succeeded.

Tests exercise delayed BUSY assertion in async and blocking modes. Optical image
quality, reset behavior and the conservative 10 ms guard still require validation
on the physical GDEM0097T61 panel; software tests cannot prove ghosting is fixed.
