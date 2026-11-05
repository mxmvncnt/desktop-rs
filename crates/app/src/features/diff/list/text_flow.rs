//! How row text is laid out (scroll vs wrap) and the row/text/marker boxes
//! shared by the unified and split row renderers.

use gd_ui::theme::DIFF_ROW_HEIGHT;
use gpui::{
    Bounds, Div, Hsla, IntoElement, PaintQuad, ParentElement, Pixels, Styled, StyledText, canvas,
    div, point, px, size,
};

use crate::features::diff::cell_text::CellText;

/// How row text is laid out. `Scroll` clips each text cell and shifts it
/// left by the shared horizontal offset (the same value for both split
/// columns, so they always move together); `Wrap` breaks long lines.
#[derive(Clone, Copy, PartialEq)]
pub enum TextFlow {
    Scroll(Pixels),
    Wrap,
}

/// Width of the `+`/`-` marker column.
pub const MARKER_W: f32 = 20.0;

/// The box of one diff row: one line tall, or at least one line when wrapping.
pub fn row_box(flow: TextFlow) -> Div {
    let d = div().flex().w_full();
    match flow {
        TextFlow::Scroll(_) => d.h(px(DIFF_ROW_HEIGHT)).whitespace_nowrap(),
        TextFlow::Wrap => d.min_h(px(DIFF_ROW_HEIGHT)),
    }
}

/// The text part of a row (after the marker), see [`TextFlow`]. `fg` colors a
/// plain line; highlighted runs carry their own colors.
///
/// The text's parent is a flex column: a block parent (the default display) would
/// collect it into its own inline paragraph, and then the `StyledText` is never
/// measured itself and its layout can't hit-test (GPUI-CE panics).
pub fn text_cell(text: StyledText, fg: Hsla, flow: TextFlow) -> Div {
    let cell = div().flex_1().min_w_0();
    match flow {
        TextFlow::Scroll(x) => cell.h_full().overflow_hidden().child(
            div()
                .relative()
                .left(-x)
                .flex()
                .flex_col()
                .text_color(fg)
                .child(text),
        ),
        TextFlow::Wrap => cell
            .whitespace_normal()
            .flex()
            .flex_col()
            .text_color(fg)
            .child(text),
    }
}

/// The `+`/`-` marker column, painted (see `CellText`). With `edge`, it also draws the
/// gutter's right border as a 1px line on its left: the gutter then needs no wrapper box,
/// and the line still lies over the row background like a border would.
pub fn marker_cell(marker: &'static str, color: Hsla, edge: Option<Hsla>) -> impl IntoElement {
    let edge_w = if edge.is_some() { px(1.0) } else { px(0.0) };
    canvas(
        |_, _, _| {},
        move |bounds, _, window, cx| {
            let edge_w = window.pixel_snap(edge_w);
            if let Some(edge) = edge {
                let line = Bounds::new(bounds.origin, size(edge_w, bounds.size.height));
                window.paint_quad(PaintQuad::new(line).background(edge));
            }
            if marker.trim().is_empty() {
                return;
            }
            if let Some(label) = CellText::shape(marker.to_string(), Some(color), window) {
                let x = label.centered_x(bounds.left() + edge_w, px(MARKER_W), window);
                label.paint(point(x, bounds.top()), window, cx);
            }
        },
    )
    .flex_none()
    .w(px(MARKER_W) + edge_w)
}
