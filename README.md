# epd-gdem0097t61

A `no_std` Rust driver for the GDEM0097T61 monochrome e-paper display, built on `embedded-hal` 1.0 traits.

The driver is platform-independent and can be used with any microcontroller platform that provides compatible SPI, GPIO, and delay implementations.

## Features

- `no_std`
- `embedded-hal` 1.0
- `SpiDevice` support for shared SPI buses
- Full and partial display refresh
- Packed 1-bit framebuffer
- Configurable BUSY timeout and polling interval
- Optional non-blocking refresh triggering
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

`Config::default()` uses a 15-second BUSY timeout, a 100-microsecond polling interval, and waits for refresh operations to finish.

```rust
use epd_gdem0097t61::Config;

let config = Config {
    busy_timeout_us: 15_000_000,
    busy_poll_interval_us: 100,
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
