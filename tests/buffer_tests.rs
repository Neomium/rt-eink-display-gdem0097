use epd_gdem0097t61::{BUFFER_SIZE, FrameBuffer, HEIGHT, ROW_BYTES, WIDTH};

#[test]
fn display_dimensions_and_buffer_size_are_correct() {
    assert_eq!(WIDTH, 88);
    assert_eq!(HEIGHT, 184);
    assert_eq!(ROW_BYTES, 11);
    assert_eq!(BUFFER_SIZE, 2024);
}

#[test]
fn new_framebuffer_is_completely_white() {
    let framebuffer = FrameBuffer::new();

    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0xFF));
    assert_eq!(framebuffer.is_black(0, 0), Some(false));
    assert_eq!(framebuffer.is_black(WIDTH - 1, HEIGHT - 1), Some(false));
}

#[test]
fn clear_fills_the_entire_framebuffer() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.clear(false);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0x00));

    framebuffer.clear(true);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0xFF));
}

#[test]
fn set_pixel_changes_only_the_requested_pixel() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(0, 0, true);

    assert_eq!(framebuffer.is_black(0, 0), Some(true));
    assert_eq!(framebuffer.is_black(1, 0), Some(false));
    assert_eq!(framebuffer.as_bytes()[0], 0b0111_1111);

    framebuffer.set_pixel(0, 0, false);

    assert_eq!(framebuffer.is_black(0, 0), Some(false));
    assert_eq!(framebuffer.as_bytes()[0], 0xFF);
}

#[test]
fn pixels_are_mapped_across_byte_boundaries_correctly() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(7, 0, true);
    framebuffer.set_pixel(8, 0, true);

    assert_eq!(framebuffer.as_bytes()[0], 0b1111_1110);
    assert_eq!(framebuffer.as_bytes()[1], 0b0111_1111);
    assert_eq!(framebuffer.is_black(7, 0), Some(true));
    assert_eq!(framebuffer.is_black(8, 0), Some(true));
}

#[test]
fn last_pixel_is_stored_in_the_last_byte() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(WIDTH - 1, HEIGHT - 1, true);

    assert_eq!(framebuffer.is_black(WIDTH - 1, HEIGHT - 1), Some(true));
    assert_eq!(framebuffer.as_bytes()[BUFFER_SIZE - 1], 0b1111_1110);
}

#[test]
fn out_of_bounds_coordinates_are_ignored() {
    let mut framebuffer = FrameBuffer::new();
    let original = framebuffer.clone();

    framebuffer.set_pixel(WIDTH, 0, true);
    framebuffer.set_pixel(0, HEIGHT, true);
    framebuffer.set_pixel(WIDTH, HEIGHT, true);

    assert_eq!(framebuffer, original);
    assert_eq!(framebuffer.is_black(WIDTH, 0), None);
    assert_eq!(framebuffer.is_black(0, HEIGHT), None);
}