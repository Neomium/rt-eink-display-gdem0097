use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::BinaryColor,
    prelude::Pixel,
};

use crate::{Color, FrameBuffer, HEIGHT, WIDTH};
use crate::graphics::LandscapeFrameBuffer;

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

#[test]
fn landscape_framebuffer_has_rotated_dimensions() {
    let mut framebuffer = FrameBuffer::new();
    let target = LandscapeFrameBuffer::new(&mut framebuffer);

    assert_eq!(
        target.size(),
        Size::new(HEIGHT as u32, WIDTH as u32)
    );
}

#[test]
fn landscape_top_left_maps_to_physical_top_right() {
    let mut framebuffer = FrameBuffer::new();

    {
        let mut target =
            LandscapeFrameBuffer::new(&mut framebuffer);

        target
            .draw_iter([Pixel(
                Point::new(0, 0),
                BinaryColor::On,
            )])
            .unwrap();
    }

    assert_eq!(
        framebuffer.color_of_pixel(WIDTH - 1, 0),
        Ok(Color::Black)
    );
}

#[test]
fn landscape_bottom_right_maps_to_physical_bottom_left() {
    let mut framebuffer = FrameBuffer::new();

    {
        let mut target =
            LandscapeFrameBuffer::new(&mut framebuffer);

        target
            .draw_iter([Pixel(
                Point::new(
                    HEIGHT as i32 - 1,
                    WIDTH as i32 - 1,
                ),
                BinaryColor::On,
            )])
            .unwrap();
    }

    assert_eq!(
        framebuffer.color_of_pixel(0, HEIGHT - 1),
        Ok(Color::Black)
    );
}

#[test]
fn landscape_framebuffer_ignores_out_of_bounds_pixels() {
    let mut framebuffer = FrameBuffer::new();

    {
        let mut target =
            LandscapeFrameBuffer::new(&mut framebuffer);

        target
            .draw_iter([
                Pixel(
                    Point::new(-1, 0),
                    BinaryColor::On,
                ),
                Pixel(
                    Point::new(HEIGHT as i32, 0),
                    BinaryColor::On,
                ),
                Pixel(
                    Point::new(0, WIDTH as i32),
                    BinaryColor::On,
                ),
            ])
            .unwrap();
    }

    assert!(
        framebuffer
            .as_bytes() 
            .iter()
            .all(|byte| *byte == 0xFF)
    );
}

#[test]
fn landscape_pixel_maps_to_expected_physical_coordinate() {
    let mut framebuffer = FrameBuffer::new();

    {
        let mut target = LandscapeFrameBuffer::new(&mut framebuffer);

        target
            .draw_iter([Pixel(
                Point::new(10, 20),
                BinaryColor::On,
            )])
            .unwrap();
    }

    assert_eq!(
        framebuffer.color_of_pixel(WIDTH - 1 - 20, 10),
        Ok(Color::Black)
    );
}