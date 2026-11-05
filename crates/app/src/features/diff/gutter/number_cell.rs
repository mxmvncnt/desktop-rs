//! One line-number cell of a diff row. A plain fn, not a `RenderOnce`: it is
//! built for every visible row every frame (hot-row exception, see
//! docs/CONVENTIONS.md).

use gd_git::{Diff, DiffLineKind};
use gd_ui::{Icon, Theme, theme::DIFF_ROW_HEIGHT};
use gpui::{
    AnyElement, App, Bounds, Div, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, Styled, TextAlign, Window, canvas, div, point, prelude::*, px, size,
};

use super::block_handle::RowSelect;
use super::style::{CHECK_MARK, CHECK_W, line_selected_bg, number_cell_w};

/// Right padding of each line number.
const NUMBER_PAD: f32 = 8.0;
use crate::features::diff::state::GutterDrag;
use crate::repo::RepoModel;

/// One line-number cell holding `numbers` (one per column: two in unified
/// view, one per side in split view). Without `select` (or for a line that
/// can't be selected) it is plain; otherwise clicking toggles `row`, dragging
/// extends, and an included row is solid blue with one check mark, like Desktop.
pub fn number_cell(
    select: Option<(&RowSelect, &RepoModel)>,
    diff: &Diff,
    row: Option<usize>,
    numbers: &[Option<u32>],
    id: (&'static str, usize),
    num_w: Pixels,
    theme: &Theme,
) -> AnyElement {
    let tint = match row.map(|ix| diff.lines[ix].kind) {
        Some(DiffLineKind::Add) => theme.diff_add_gutter_bg,
        Some(DiffLineKind::Delete) => theme.diff_del_gutter_bg,
        Some(DiffLineKind::Hunk) => theme.diff_hunk_gutter_bg,
        _ => theme.diff_gutter_bg,
    };
    let selectable = row.filter(|&ix| gd_git::selectable(diff, ix));
    let included = select
        .zip(selectable)
        .is_some_and(|((_, r), ix)| r.row_included(ix));
    let cell = cell_shell(select.is_some(), included, numbers, num_w, theme);
    let (Some((sel, _)), Some(ix)) = (select, selectable) else {
        return cell.bg(tint).into_any_element();
    };
    selectable_cell(cell, sel, ix, id, included, tint, theme)
}

/// The cell box: optional check slot, then one right-aligned number per column.
/// Top-aligned so numbers stay on the first line of a wrapped row. No
/// `h_full`: a percentage can't resolve in an auto-height (wrapped) row,
/// and it would stop the cell from stretching to the row's height.
///
/// The check and numbers are painted by one canvas, where they used to be five
/// small elements: laying those out in every visible row was the largest part of
/// a diff scroll frame (docs/ARCHITECTURE.md, "Measuring scroll cost"). The
/// canvas places them exactly where the flex layout did.
fn cell_shell(
    has_select: bool,
    included: bool,
    numbers: &[Option<u32>],
    num_w: Pixels,
    theme: &Theme,
) -> Div {
    let check_w = if has_select { px(CHECK_W) } else { px(0.0) };
    let check = (has_select && included).then_some(theme.on_accent);
    let width = number_cell_w(num_w, numbers.len() as f32, has_select);
    let numbers = numbers.to_vec();
    let paint = move |bounds: Bounds<Pixels>, _, window: &mut Window, cx: &mut App| {
        if let Some(color) = check {
            let mark = px(CHECK_MARK);
            let origin = bounds.origin
                + point(
                    (px(CHECK_W) - mark) / 2.0,
                    (px(DIFF_ROW_HEIGHT) - mark) / 2.0,
                );
            let at = Bounds::new(origin, size(mark, mark));
            let path = Icon::Check.path().into();
            window
                .paint_svg(at, path, None, Default::default(), color, cx)
                .ok();
        }
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.pixel_snap(
            style
                .line_height
                .to_pixels(font_size.into(), window.rem_size()),
        );
        // Snapped like gpui's layout: lengths to the nearest device pixel, a measured
        // text width up to the next one.
        let (check_w, num_w, pad) = (
            window.pixel_snap(check_w),
            window.pixel_snap(num_w),
            window.pixel_snap(px(NUMBER_PAD)),
        );
        let scale = window.scale_factor();
        for (i, n) in numbers.iter().enumerate() {
            let Some(n) = n else { continue };
            let text = n.to_string();
            let runs = [style.to_run(text.len())];
            let Some(shaped) = window
                .text_system()
                .shape_text(text, font_size, &runs, None, None)
                .ok()
            else {
                continue;
            };
            let width = px((shaped.size(line_height).width.as_f32() * scale).ceil() / scale);
            let right = bounds.left() + check_w + num_w * (i + 1) as f32 - pad;
            let origin = point(right - width, bounds.top());
            shaped
                .paint(origin, line_height, TextAlign::Left, None, window, cx)
                .ok();
        }
    };
    div()
        .flex()
        .flex_none()
        .items_start()
        .text_color(theme.diff_gutter)
        .child(canvas(|_, _, _| {}, paint).w(width).h(px(DIFF_ROW_HEIGHT)))
}

fn selectable_cell(
    cell: Div,
    sel: &RowSelect,
    ix: usize,
    id: (&'static str, usize),
    included: bool,
    tint: gpui::Hsla,
    theme: &Theme,
) -> AnyElement {
    let (repo_down, ws_down) = (sel.repo.clone(), sel.ws.clone());
    let (repo_move, ws_move) = (sel.repo.clone(), sel.ws.clone());
    let ws_menu = sel.ws.clone();
    cell.id(id)
        .cursor_pointer()
        .map(|d| {
            if included {
                let hover = theme.accent;
                d.bg(line_selected_bg())
                    .text_color(theme.on_accent)
                    .hover_style(move |s| s.bg(hover))
            } else {
                let hover = theme.text_muted;
                d.bg(tint)
                    .hover_style(move |s| s.bg(theme.ink(0.14)).text_color(hover))
            }
        })
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            let include = repo_down.update(cx, |r, cx| r.toggle_row(ix, cx));
            ws_down.update(cx, |w, _| {
                w.diff_select.drag = Some(GutterDrag {
                    anchor: ix,
                    include,
                })
            });
        })
        .on_mouse_down(MouseButton::Right, move |e, w, cx| {
            ws_menu.update(cx, |this, cx| {
                this.open_discard_rows_menu(vec![ix], false, e.position, w, cx)
            })
        })
        .on_mouse_move(move |e, _, cx| {
            if e.pressed_button != Some(MouseButton::Left) {
                return;
            }
            if let Some(GutterDrag { anchor, include }) = ws_move.read(cx).diff_select.drag {
                repo_move.update(cx, |r, cx| {
                    r.set_rows_included(anchor.min(ix)..=anchor.max(ix), include, cx)
                });
            }
        })
        .into_any_element()
}
