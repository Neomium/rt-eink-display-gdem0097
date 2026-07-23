# epd-gdem0097t61

A no_std Rust driver for the GDEM0097T61 monochrome e-paper display, built on embedded-hal 1.0 traits.

The driver is platform-independent and can be used with any microcontroller platform that provides compatible embedded-hal implementations.

---

## Features

- `no_std`
- Built on `embedded-hal` 1.0
- Hardware-independent
- Full display refresh
- Partial display refresh
- Packed 1-bit framebuffer
- Busy timeout handling
- Deep sleep support
---

## Supported Hardware

This crate currently supports:

| Display | Resolution | Colors |
|---------|------------|--------|
| GDEM0097T61 | 88 × 184 | Black / White |

---

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
epd-gdem0097t61 = "0.1"
```

---

## Full refresh example

Create the driver using your platform-specific SPI peripheral, GPIO pins, and delay implementation.

```rust
use epd_gdem0097t61::{Color, EpdDisplay, FrameBuffer};

let mut display = EpdDisplay::new(
    spi,
    cs,
    dc,
    rst,
    busy,
    delay,
)?;

display.init()?;

let mut framebuffer = FrameBuffer::new();

framebuffer.clear(Color::White);
framebuffer.set_pixel(10, 20, Color::Black);

display.show_full(framebuffer.as_bytes())?;

display.sleep()?;
```

The concrete types of spi, cs, dc, rst, busy, and delay depend on the microcontroller and hardware abstraction layer being used.

## Partial refresh example

A partial refresh updates only a selected rectangular region.

```rust
use epd_gdem0097t61::{Color, EpdDisplay, FrameBuffer, Region};

let mut framebuffer = FrameBuffer::new();

framebuffer.set_pixel(16, 20, Color::Black);

let region = Region::new(
    16, // x
    16, // y
    32, // width
    24, // height
);

display.show_partial(&framebuffer, region)?;

A Region is defined as:

Region::new(x, y, width, height)
```

The coordinates use the display's normal framebuffer coordinate system:

(0, 0) is the top-left pixel;
X increases from left to right;
Y increases from top to bottom.

## Partial-refresh restrictions

The framebuffer stores eight horizontal pixels in each byte. Partial regions must therefore be byte-aligned.

The following conditions apply:

- x must be divisible by 8;
- width must be divisible by 8;
- width and height must be greater than zero;
- the region must remain inside the display boundaries;
- the end-coordinate calculations must not overflow.

## Framebuffer

The display uses a packed 1-bit framebuffer.

- Black pixels are stored as cleared bits: 0
- White pixels are stored as set bits: 1
- Each byte stores eight horizontal pixels
- The most significant bit represents the leftmost pixel in a byte

The framebuffer dimensions are:

- Width = 88 pixels 
- Height = 184 pixels 
- Bytes/row = 11 bytes 
- Buffer size = 2024 bytes 

Pixel values can be read using:

```rust
let color = framebuffer.pixel(20, 30);
```

## Public API

The crate exposes:

```rust
EpdDisplay
FrameBuffer
Color
Region
WIDTH
HEIGHT
ROW_BYTES
BUFFER_SIZE
EpdError
```

---




