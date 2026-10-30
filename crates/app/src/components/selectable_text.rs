//! Selectable one-line texts outside the diff (commit header, file name bars). They share the diff's
//! `TextSel` model and `Workspace::commit_sel`; `texts` are the two lines the selection is made on.

use gd_ui::Theme;
use gpui::{
    App, Div, Entity, FontWeight, HighlightStyle, Hsla, MouseButton, MouseMoveEvent, StyledText, TextLayout, Window, div,
    prelude::*,
};

use crate::features::diff::text_selection::model::{TextPos, TextSel, TextSide};
use crate::workspace::Workspace;

/// Selectable text of `line` in `texts` with its selection highlighted, and its layout for hit-testing.
pub fn selectable_line(
    ws: &Entity<Workspace>,
    texts: &[String; 2],
    line: usize,
    sel: Option<TextSel>,
    fg: Hsla,
    weight: FontWeight,
    theme: &Theme,
) -> (Div, TextLayout) {
    let text = &texts[line];
    let highlight = HighlightStyle { background_color: Some(theme.selection), ..Default::default() };
    let cols = sel.and_then(|s| s.cols_in(line, TextSide::Unified, text.len()));
    let styled = StyledText::new(text.clone()).with_highlights(cols.map(|r| (r, highlight)));
    let layout = styled.layout().clone();
    let (ws, hit, texts) = (ws.clone(), layout.clone(), texts.clone());
    // The text sits in a flex column so it is measured and can hit-test (see `diff::highlight::line_text`).
    let cell = div().flex().flex_col().cursor_text().text_color(fg).font_weight(weight).child(styled).on_mouse_down(
        MouseButton::Left,
        move |e, window, cx| {
            let col = hit.index_for_position(e.position).unwrap_or_else(|nearest| nearest);
            let (clicks, shift) = (e.click_count, e.modifiers.shift);
            ws.update(cx, |w, cx| w.commit_text_down(texts.clone(), TextPos { line, col }, clicks, shift, window, cx));
        },
    );
    (cell, layout)
}

/// Mouse-move handler for the container of `layouts` (one per line): while dragging, the pointer moves
/// the caret to the nearest text position in the line it is on.
pub fn drag_selection(
    ws: Entity<Workspace>,
    layouts: Vec<TextLayout>,
) -> impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static {
    move |e, _, cx| {
        if e.pressed_button != Some(MouseButton::Left) || !ws.read(cx).commit_sel.drag {
            return;
        }
        let line = layouts.iter().position(|l| e.position.y < l.bounds().bottom()).unwrap_or(layouts.len() - 1);
        let col = layouts[line].index_for_position(e.position).unwrap_or_else(|nearest| nearest);
        ws.update(cx, |w, cx| w.commit_text_drag(TextPos { line, col }, cx));
    }
}
