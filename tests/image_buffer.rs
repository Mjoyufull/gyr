//! Graphics payloads must not consume neighboring text during Ratatui diffing.

use image::{DynamicImage, Rgba, RgbaImage};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::widgets::Widget;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::{Image, Resize};

#[test]
fn image_payloads_do_not_skip_labels_or_later_rows() {
    for protocol_type in [
        ProtocolType::Sixel,
        ProtocolType::Kitty,
        ProtocolType::Halfblocks,
    ] {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(protocol_type);
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 32, Rgba([0, 80, 220, 255])));
        let protocol = picker
            .new_protocol(image, Size::new(2, 2), Resize::Fit(None))
            .expect("fixture image should encode");
        let blank = Buffer::empty(Rect::new(0, 0, 40, 8));
        let mut next = blank.clone();
        Image::new(&protocol).render(Rect::new(1, 1, 2, 2), &mut next);
        next[(5, 1)].set_symbol("L");
        next[(0, 6)].set_symbol(">");
        next[(5, 6)].set_symbol("Z");
        let changes: Vec<_> = blank.diff_iter(&next).map(|(x, y, _)| (x, y)).collect();
        for position in [(5, 1), (0, 6), (5, 6)] {
            assert!(
                changes.contains(&position),
                "{protocol_type:?} skipped {position:?}"
            );
        }
        let erased: Vec<_> = next.diff_iter(&blank).map(|(x, y, _)| (x, y)).collect();
        assert!(
            erased.contains(&(1, 1)),
            "{protocol_type:?} left its image anchor behind"
        );
        assert!(
            erased.contains(&(5, 6)),
            "{protocol_type:?} left its label behind"
        );
    }
}
