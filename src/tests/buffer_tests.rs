use epd_gdem0097t61::{BUFFER_SIZE, Color, FrameBuffer, HEIGHT, ROW_BYTES, WIDTH, Region};

mod framebuffer_tests {
    use super::*;
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
    assert_eq!(framebuffer.pixel(0, 0), Some(Color::White));
    assert_eq!(framebuffer.pixel(WIDTH - 1, HEIGHT - 1), Some(Color::White));
}

#[test]
fn clear_fills_the_entire_framebuffer() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.clear(Color::Black);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0x00));

    framebuffer.clear(Color::White);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == 0xFF));
}

#[test]
fn set_pixel_changes_only_the_requested_pixel() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(0, 0, Color::Black);

    assert_eq!(framebuffer.pixel(0, 0), Some(Color::Black));
    assert_eq!(framebuffer.pixel(1, 0), Some(Color::White));
    assert_eq!(framebuffer.as_bytes()[0], 0b0111_1111);

    framebuffer.set_pixel(0, 0, Color::White);
    assert_eq!(framebuffer.pixel(0, 0), Some(Color::White));
    assert_eq!(framebuffer.as_bytes()[0], 0xFF);
}

#[test]
fn pixels_are_mapped_across_byte_boundaries_correctly() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(7, 0, Color::Black);
    framebuffer.set_pixel(8, 0, Color::Black);

    assert_eq!(framebuffer.as_bytes()[0], 0b1111_1110);
    assert_eq!(framebuffer.as_bytes()[1], 0b0111_1111);
    assert_eq!(framebuffer.pixel(7, 0), Some(Color::Black));
    assert_eq!(framebuffer.pixel(8, 0), Some(Color::Black));
}

#[test]
fn last_pixel_is_stored_in_the_last_byte() {
    let mut framebuffer = FrameBuffer::new();

    framebuffer.set_pixel(WIDTH - 1, HEIGHT - 1, Color::Black);

    assert_eq!(framebuffer.pixel(WIDTH - 1, HEIGHT - 1), Some(Color::Black));
    assert_eq!(framebuffer.as_bytes()[BUFFER_SIZE - 1], 0b1111_1110);
}

#[test]
fn out_of_bounds_coordinates_are_ignored() {
    let mut framebuffer = FrameBuffer::new();
    let original = framebuffer.clone();

    framebuffer.set_pixel(WIDTH, 0, Color::Black);
    framebuffer.set_pixel(0, HEIGHT, Color::Black);
    framebuffer.set_pixel(WIDTH, HEIGHT, Color::Black);

    assert_eq!(framebuffer, original);
    assert_eq!(framebuffer.pixel(WIDTH, 0), None);
    assert_eq!(framebuffer.pixel(0, HEIGHT), None);
}
}

mod region_tests {
    use super::*;

    #[test]
    fn region_stores_the_provided_values() {
        let region = Region::new(16, 20, 32, 10);

        assert_eq!(region.x, 16);
        assert_eq!(region.y, 20);
        assert_eq!(region.width, 32);
        assert_eq!(region.height, 10);
    }

    #[test]
    fn region_end_coordinates_are_correct() {
        let region = Region::new(16, 20, 32, 10);

        assert_eq!(region.x_end(), Some(48));
        assert_eq!(region.y_end(), Some(30));
    }

    #[test]
    fn region_end_returns_none_on_overflow() {
        let region = Region::new(
            usize::MAX,
            usize::MAX,
            1,
            1,
        );

        assert_eq!(region.x_end(), None);
        assert_eq!(region.y_end(), None);
    }
}