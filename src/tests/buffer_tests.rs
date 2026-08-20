use crate::{
    BUFFER_SIZE, Color, FrameBuffer, FrameBufferError, HEIGHT, ROW_BYTES, Region, RegionError,
    WIDTH,
};

fn assert_region_new_error(x: usize, y: usize, width: usize, height: usize, expected: RegionError) {
    assert_eq!(Region::new(x, y, width, height), Err(expected));
}

#[test]
fn framebuffer_constants_have_the_expected_dimensions_and_size() {
    assert_eq!(WIDTH, 88);
    assert_eq!(HEIGHT, 184);
    assert_eq!(ROW_BYTES, 11);
    assert_eq!(BUFFER_SIZE, 2024);
}

#[test]
fn framebuffer_new_creates_a_completely_white_buffer() {
    let framebuffer = FrameBuffer::new();

    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0xFF));
    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::White));
    assert_eq!(
        framebuffer.color_of_pixel(WIDTH - 1, HEIGHT - 1),
        Ok(Color::White)
    );
}

#[test]
fn framebuffer_filled_uses_the_requested_color() {
    let black = FrameBuffer::filled(Color::Black);
    let white = FrameBuffer::filled(Color::White);

    assert!(black.as_bytes().iter().all(|byte| *byte == 0x00));
    assert!(white.as_bytes().iter().all(|byte| *byte == 0xFF));

    assert_eq!(black.color_of_pixel(0, 0), Ok(Color::Black));
    assert_eq!(white.color_of_pixel(0, 0), Ok(Color::White));
}

#[test]
fn framebuffer_clear_fills_the_entire_buffer() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.clear(Color::Black);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0x00));

    framebuffer.clear(Color::White);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0xFF));
}

#[test]
fn framebuffer_set_pixel_changes_only_the_requested_pixel() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(0, 0, Color::Black).unwrap();

    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::Black));
    assert_eq!(framebuffer.color_of_pixel(1, 0), Ok(Color::White));
    assert_eq!(framebuffer.as_bytes()[0], 0b0111_1111);

    framebuffer.set_pixel(0, 0, Color::White).unwrap();
    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::White));
    assert_eq!(framebuffer.as_bytes()[0], 0xFF);
}

#[test]
fn framebuffer_set_pixel_preserves_other_pixels_in_the_same_byte() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(0, 0, Color::Black).unwrap();
    framebuffer.set_pixel(3, 0, Color::Black).unwrap();

    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::Black));
    assert_eq!(framebuffer.color_of_pixel(3, 0), Ok(Color::Black));
    assert_eq!(framebuffer.color_of_pixel(1, 0), Ok(Color::White));
    assert_eq!(framebuffer.as_bytes()[0], 0b0110_1111);
}

#[test]
fn framebuffer_set_pixel_maps_pixels_across_byte_boundaries() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(7, 0, Color::Black).unwrap();
    framebuffer.set_pixel(8, 0, Color::Black).unwrap();

    assert_eq!(framebuffer.as_bytes()[0], 0b1111_1110);
    assert_eq!(framebuffer.as_bytes()[1], 0b0111_1111);
    assert_eq!(framebuffer.color_of_pixel(7, 0), Ok(Color::Black));
    assert_eq!(framebuffer.color_of_pixel(8, 0), Ok(Color::Black));
}

#[test]
fn framebuffer_set_pixel_stores_pixels_in_the_correct_rows() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(0, 1, Color::Black).unwrap();

    assert_eq!(framebuffer.as_bytes()[0], 0xFF);
    assert_eq!(framebuffer.as_bytes()[ROW_BYTES], 0b0111_1111);
    assert_eq!(framebuffer.color_of_pixel(0, 0), Ok(Color::White));
    assert_eq!(framebuffer.color_of_pixel(0, 1), Ok(Color::Black));
}

#[test]
fn framebuffer_set_pixel_stores_the_last_pixel_in_the_last_byte() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer
        .set_pixel(WIDTH - 1, HEIGHT - 1, Color::Black)
        .unwrap();

    assert_eq!(
        framebuffer.color_of_pixel(WIDTH - 1, HEIGHT - 1),
        Ok(Color::Black)
    );
    assert_eq!(framebuffer.as_bytes()[BUFFER_SIZE - 1], 0b1111_1110);
}

#[test]
fn framebuffer_set_pixel_returns_error_for_out_of_bounds_coordinates() {
    let mut framebuffer = FrameBuffer::new();
    let original = framebuffer.clone();

    assert_eq!(
        framebuffer.set_pixel(WIDTH, 0, Color::Black),
        Err(FrameBufferError::OutOfBounds)
    );
    assert_eq!(
        framebuffer.set_pixel(0, HEIGHT, Color::Black),
        Err(FrameBufferError::OutOfBounds)
    );
    assert_eq!(
        framebuffer.set_pixel(WIDTH, HEIGHT, Color::Black),
        Err(FrameBufferError::OutOfBounds)
    );

    assert_eq!(framebuffer, original);

    assert_eq!(
        framebuffer.color_of_pixel(WIDTH, 0),
        Err(FrameBufferError::OutOfBounds)
    );
    assert_eq!(
        framebuffer.color_of_pixel(0, HEIGHT),
        Err(FrameBufferError::OutOfBounds)
    );
}

#[test]
fn framebuffer_set_pixel_returns_error_for_extreme_out_of_bounds_coordinates() {
    let mut framebuffer = FrameBuffer::new();
    let original = framebuffer.clone();

    assert_eq!(
        framebuffer.set_pixel(usize::MAX, 0, Color::Black),
        Err(FrameBufferError::OutOfBounds)
    );

    assert_eq!(
        framebuffer.set_pixel(0, usize::MAX, Color::Black),
        Err(FrameBufferError::OutOfBounds)
    );

    assert_eq!(framebuffer, original);
}

#[test]
fn region_new_stores_the_provided_values() {
    let region = Region::new(16, 20, 32, 10).unwrap();

    assert_eq!(region.x(), 16);
    assert_eq!(region.y(), 20);
    assert_eq!(region.width(), 32);
    assert_eq!(region.height(), 10);
}

#[test]
fn region_new_produces_the_expected_end_coordinates() {
    let region = Region::new(16, 20, 32, 10).unwrap();

    assert_eq!(region.x() + region.width(), 48);
    assert_eq!(region.y() + region.height(), 30);
}

#[test]
fn region_new_rejects_overflowing_coordinates() {
    let x_error = Region::new(usize::MAX - 7, 0, 8, 1).unwrap_err();

    let y_error = Region::new(0, usize::MAX, 8, 1).unwrap_err();

    assert_eq!(x_error, RegionError::Invalid);
    assert_eq!(y_error, RegionError::Invalid);
}
#[test]
fn region_new_rejects_zero_sized_regions() {
    assert_region_new_error(0, 0, 0, 8, RegionError::Invalid);
    assert_region_new_error(0, 0, 8, 0, RegionError::Invalid);
}

#[test]
fn region_new_rejects_out_of_bounds_regions() {
    assert_region_new_error(WIDTH, 0, 8, 8, RegionError::Invalid);
    assert_region_new_error(0, HEIGHT, 8, 8, RegionError::Invalid);
    assert_region_new_error(WIDTH - 8, 0, 16, 8, RegionError::Invalid);
}

#[test]
fn region_new_rejects_non_byte_aligned_regions() {
    assert_region_new_error(1, 0, 8, 8, RegionError::Unaligned);
    assert_region_new_error(0, 0, 7, 8, RegionError::Unaligned);
}
