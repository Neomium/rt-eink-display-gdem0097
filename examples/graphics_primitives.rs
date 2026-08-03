use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};

use epd_gdem0097t61::FrameBuffer;

fn main() {
    let mut framebuffer = FrameBuffer::new();

    Rectangle::new(Point::new(4, 4), Size::new(40, 20))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(&mut framebuffer)
        .unwrap();

    Circle::new(Point::new(50, 20), 24)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut framebuffer)
        .unwrap();

    // Drawing only changes framebuffer memory.
    //
    // On real hardware:
    // display.show_full(&framebuffer).unwrap();
}
