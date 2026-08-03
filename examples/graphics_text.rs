use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

use epd_gdem0097t61::FrameBuffer;

fn main() {
    let mut framebuffer = FrameBuffer::new();

    let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    Text::new("Hello e-paper!", Point::new(4, 20), text_style)
        .draw(&mut framebuffer)
        .unwrap();

    Text::new("88 x 184", Point::new(4, 40), text_style)
        .draw(&mut framebuffer)
        .unwrap();

    // Transfer the completed framebuffer separately:
    //
    // display.show_full(&framebuffer).unwrap();
}
