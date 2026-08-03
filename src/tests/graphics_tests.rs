use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::BinaryColor,
    prelude::Pixel,
};

use crate::{Color, FrameBuffer, HEIGHT, WIDTH};

#[test]
fn origin_dimensions_size_returns_display_dimensions() {
    let framebuffer = FrameBuffer::new();

    assert_eq!(framebuffer.size(), Size::new(WIDTH as u32, HEIGHT as u32));
}

#[test]
fn draw_iter_draws_binary_color_pixels_into_framebuffer() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer
        .draw_iter([
            Pixel(Point::new(0, 0), BinaryColor::On),
            Pixel(Point::new(1, 0), BinaryColor::Off),
            Pixel(Point::new(10, 20), BinaryColor::On),
        ])
        .unwrap();

    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::Black));
    assert_eq!(framebuffer.color_of_pixel(1, 0), Ok(Color::White));
    assert_eq!(framebuffer.color_of_pixel(10, 20), Ok(Color::Black));
}

#[test]
fn draw_iter_ignores_pixels_outside_framebuffer() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer
        .draw_iter([
            Pixel(Point::new(-1, 0), BinaryColor::On),
            Pixel(Point::new(0, -1), BinaryColor::On),
            Pixel(Point::new(WIDTH as i32, 0), BinaryColor::On),
            Pixel(Point::new(0, HEIGHT as i32), BinaryColor::On),
        ])
        .unwrap();

    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::White));
}

#[test]
fn clear_fills_framebuffer_using_binary_color() {
    let mut framebuffer = FrameBuffer::new();

    DrawTarget::clear(&mut framebuffer, BinaryColor::On).unwrap();

    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::Black));
    assert_eq!(
        framebuffer.color_of_pixel(WIDTH - 1, HEIGHT - 1),
        Ok(Color::Black)
    );
}
