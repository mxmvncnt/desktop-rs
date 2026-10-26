//! Sizes and colors shared by the diff gutters (number cells, handle column,
//! expand buttons).

use gd_ui::{Icon, Theme, icon};
use gpui::{Hsla, IntoElement, Pixels, Styled, px};

/// Width of the check mark slot in a selectable number cell.
pub const CHECK_W: f32 = 16.0;
/// Width of the hunk handle column.
pub const HANDLE_W: f32 = 16.0;

/// Background of the idle handle column: clearly darker than the numbers.
pub fn handle_bg(theme: &Theme) -> Hsla {
    if theme.light { theme.ink(0.2) } else { gpui::ColorExt::opacity(&gpui::black(), 0.6) }
}

/// Background of an included line's number cell (blue-700).
pub fn line_selected_bg() -> Hsla {
    gpui::rgb_to_hsla(gpui::rgb(0x1447e6))
}

pub fn check_mark(color: Hsla) -> impl IntoElement {
    icon(Icon::Check).size(px(11.0)).text_color(color)
}

/// Width of a number cell holding `numbers` numbers.
pub fn number_cell_w(num_w: Pixels, numbers: f32, select: bool) -> Pixels {
    num_w * numbers + if select { px(CHECK_W) } else { px(0.0) }
}
