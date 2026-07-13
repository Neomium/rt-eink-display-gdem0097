# epd-gdem0097t61

A `no_std` Rust driver for the **GDEM0097T61** monochrome e-paper display using `embedded-hal` 1.0 traits.

This crate provides a lightweight, synchronous interface for initializing the display, managing a framebuffer, writing image data, refreshing the screen, and entering deep sleep. It is platform-independent and can be used with any microcontroller that provides `embedded-hal` implementations.

---

## Features

- `no_std`
- Built on `embedded-hal` 1.0
- Hardware-independent
- Full display refresh
- 1-bit framebuffer implementation
- Pixel-level drawing API
- Deep sleep support
- No dynamic memory allocation

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

## Usage

Create the driver using your platform's SPI peripheral and GPIO pins.

```rust
use epd_gdem0097t61::{Color, EpdDisplay, FrameBuffer};

// Create your SPI peripheral, GPIO pins and delay implementation.
// These are platform specific.

let mut display = EpdDisplay::new(
    spi,
    cs,
    dc,
    rst,
    busy,
)?;

display.init(&mut delay)?;

let mut framebuffer = FrameBuffer::new();

framebuffer.clear(Color::White);

framebuffer.set_pixel(10, 20, Color::Black);

display.show_full(framebuffer.as_bytes(), &mut delay)?;

display.sleep()?;
```

---

## Framebuffer

The display uses a packed 1-bit framebuffer.

- Black pixels are stored as cleared bits (`0`)
- White pixels are stored as set bits (`1`)

The framebuffer size is:

```text
88 × 184 pixels
2024 bytes
```

Example:

```rust
let mut framebuffer = FrameBuffer::new();

framebuffer.clear(Color::White);

framebuffer.set_pixel(0, 0, Color::Black);
framebuffer.set_pixel(20, 30, Color::Black);
```

---

## Public API

The crate exposes:

```rust
EpdDisplay
FrameBuffer
Color
WIDTH
HEIGHT
ROW_BYTES
BUFFER_SIZE
EpdError
```

---

## Error Handling

Operations return:

```rust
Result<T, EpdError<_>>
```

Possible errors include:

- SPI communication failure
- GPIO failure
- Busy timeout





