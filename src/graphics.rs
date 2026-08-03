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
