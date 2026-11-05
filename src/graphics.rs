use core::convert::Infallible;

use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    prelude::Pixel,
};

use crate::{Color, FrameBuffer, HEIGHT, WIDTH};

impl OriginDimensions for FrameBuffer {
    #[allow(clippy::cast_possible_truncation)]
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for FrameBuffer {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            let Ok(x) = usize::try_from(point.x) else {
                continue;
            };

            let Ok(y) = usize::try_from(point.y) else {
                continue;
            };

            if x >= WIDTH || y >= HEIGHT {
                continue;
            }

            let framebuffer_color = match color {
                BinaryColor::On => Color::Black,
                BinaryColor::Off => Color::White,
            };

            self.set_pixel(x, y, framebuffer_color)
                .expect("pixel coordinates were validated");
        }

        Ok(())
    }
}

/// Landscape-oriented drawing view over a physical framebuffer.
///
/// The underlying framebuffer remains 88 × 184, while this wrapper exposes
/// logical drawing dimensions of 184 × 88.
pub struct LandscapeFrameBuffer<'a> {
    framebuffer: &'a mut FrameBuffer,
}

impl<'a> LandscapeFrameBuffer<'a> {
    #[must_use]
    pub fn new(framebuffer: &'a mut FrameBuffer) -> Self {
        Self { framebuffer }
    }
}

impl OriginDimensions for LandscapeFrameBuffer<'_> {
    #[allow(clippy::cast_possible_truncation)]
    fn size(&self) -> Size {
        Size::new(HEIGHT as u32, WIDTH as u32)
    }
}

impl DrawTarget for LandscapeFrameBuffer<'_> {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            let Ok(x) = usize::try_from(point.x) else {
                continue;
            };

            let Ok(y) = usize::try_from(point.y) else {
                continue;
            };

            if x >= HEIGHT || y >= WIDTH {
                continue;
            }

            let physical_x = WIDTH - 1 - y;
            let physical_y = x;

            let framebuffer_color = match color {
                BinaryColor::On => Color::Black,
                BinaryColor::Off => Color::White,
            };

            self.framebuffer
                .set_pixel(physical_x, physical_y, framebuffer_color)
                .expect("pixel coordinates were validated");
        }

        Ok(())
    }
}
