//! `CellText`: a short label of a diff row (line number, `+`/`-` marker) shaped for a
//! canvas. A text element per label is costly to lay out in every visible row
//! (docs/ARCHITECTURE.md, "Measuring scroll cost"); a canvas painting the label at the
//! spot gpui's layout would give it keeps the pixels identical.

use gpui::{App, Hsla, Pixels, Point, ShapedText, TextAlign, Window, px};

pub struct CellText {
    shaped: ShapedText,
    line_height: Pixels,
    /// Rounded up to a device pixel, as gpui does for a measured text.
    pub width: Pixels,
}

impl CellText {
    /// `text` in the current text style, recolored with `color` if given.
    pub fn shape(text: String, color: Option<Hsla>, window: &mut Window) -> Option<Self> {
        let style = window.text_style();
        let rem = window.rem_size();
        let font_size = style.font_size.to_pixels(rem);
        let line_height = window.pixel_snap(style.line_height.to_pixels(font_size.into(), rem));
        let mut run = style.to_run(text.len());
        if let Some(color) = color {
            run.color = color;
        }
        let shaped = window
            .text_system()
            .shape_text(text, font_size, &[run], None, None)
            .ok()?;
        let scale = window.scale_factor();
        let width = px((shaped.size(line_height).width.as_f32() * scale).ceil() / scale);
        Some(Self {
            shaped,
            line_height,
            width,
        })
    }

    /// Left edge of the label centered in a box `box_w` wide starting at `left`, with gpui's
    /// rounding of a centered child (half a device pixel goes left).
    pub fn centered_x(&self, left: Pixels, box_w: Pixels, window: &Window) -> Pixels {
        let scale = window.scale_factor();
        let free = (window.pixel_snap(box_w) - self.width).as_f32() * scale / 2.0;
        left + px((free - 0.5).ceil().max(0.0) / scale)
    }

    pub fn paint(&self, origin: Point<Pixels>, window: &mut Window, cx: &mut App) {
        self.shaped
            .paint(origin, self.line_height, TextAlign::Left, None, window, cx)
            .ok();
    }
}
