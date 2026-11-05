//! One line-number cell of a diff row. Built for every visible row every frame
//! (hot-row exception, see docs/CONVENTIONS.md), so not a `RenderOnce`.

use gd_git::{Diff, DiffLineKind};
use gd_ui::{Icon, Theme, theme::DIFF_ROW_HEIGHT};
use gpui::{
    AnyElement, App, Bounds, Canvas, Div, Hsla, InteractiveElement, IntoElement, MouseButton,
    PaintQuad, ParentElement, Pixels, Styled, Window, canvas, div, point, prelude::*, px, size,
};

use super::block_handle::RowSelect;
use super::style::{CHECK_MARK, CHECK_W, line_selected_bg, number_cell_w};
use crate::features::diff::cell_text::CellText;
use crate::features::diff::state::GutterDrag;
use crate::repo::RepoModel;

/// Right padding of each line number.
const NUMBER_PAD: f32 = 8.0;

/// One line-number cell holding `numbers` (one per column: two in unified
/// view, one per side in split view). Without `select` (or for a line that
/// can't be selected) it is plain; otherwise clicking toggles `row`, dragging
/// extends, and an included row is solid blue with one check mark, like Desktop.
pub struct NumberCell<'a> {
    pub select: Option<(&'a RowSelect, &'a RepoModel)>,
    pub diff: &'a Diff,
    pub row: Option<usize>,
    pub numbers: &'a [Option<u32>],
    pub id: (&'static str, usize),
    pub num_w: Pixels,
    /// The gutter's borders (left, right): 1px lines painted just outside the
    /// cell, over the row background, so the gutter needs no box to draw them.
    pub edges: [bool; 2],
    pub theme: &'a Theme,
}

impl NumberCell<'_> {
    pub fn build(self) -> AnyElement {
        let (diff, theme) = (self.diff, self.theme);
        let tint = match self.row.map(|ix| diff.lines[ix].kind) {
            Some(DiffLineKind::Add) => theme.diff_add_gutter_bg,
            Some(DiffLineKind::Delete) => theme.diff_del_gutter_bg,
            Some(DiffLineKind::Hunk) => theme.diff_hunk_gutter_bg,
            _ => theme.diff_gutter_bg,
        };
        let selectable = self.row.filter(|&ix| gd_git::selectable(diff, ix));
        let included = self
            .select
            .zip(selectable)
            .is_some_and(|((_, r), ix)| r.row_included(ix));
        let [edge_l, edge_r] = self.edges;
        let (Some((sel, _)), Some(ix)) = (self.select, selectable) else {
            // Not clickable: the canvas alone, painting the background too.
            return self
                .canvas(false, Some(tint))
                .when(edge_l, |d| d.ml(px(1.0)))
                .when(edge_r, |d| d.mr(px(1.0)))
                .into_any_element();
        };
        let cell = div()
            .flex()
            .flex_none()
            .when(edge_l, |d| d.ml(px(1.0)))
            .when(edge_r, |d| d.mr(px(1.0)))
            .text_color(theme.diff_gutter)
            .child(self.canvas(included, None));
        selectable_cell(cell, sel, ix, self.id, included, tint, theme)
    }

    /// Optional check slot, then one right-aligned number per column, painted
    /// (see `CellText`) on a canvas that stretches to the row's height. Numbers
    /// stay on the first line of a wrapped row. With `plain_bg` the canvas is the
    /// whole cell: it paints that background and colors the numbers itself.
    fn canvas(&self, included: bool, plain_bg: Option<Hsla>) -> Canvas<()> {
        let has_select = self.select.is_some();
        let check_w = if has_select { px(CHECK_W) } else { px(0.0) };
        let check = (has_select && included).then_some(self.theme.on_accent);
        let width = number_cell_w(self.num_w, self.numbers.len() as f32, has_select);
        let (numbers, num_w, [edge_l, edge_r]) = (self.numbers.to_vec(), self.num_w, self.edges);
        let (edge_color, number_color) =
            (self.theme.border, plain_bg.map(|_| self.theme.diff_gutter));
        let paint = move |bounds: Bounds<Pixels>, _, window: &mut Window, cx: &mut App| {
            if let Some(bg) = plain_bg {
                window.paint_quad(PaintQuad::new(bounds).background(bg));
            }
            // Lengths snapped to the device pixel grid, like gpui's layout.
            let edge = window.pixel_snap(px(1.0));
            for (on, x) in [(edge_l, bounds.left() - edge), (edge_r, bounds.right())] {
                if on {
                    let line = Bounds::new(point(x, bounds.top()), size(edge, bounds.size.height));
                    window.paint_quad(PaintQuad::new(line).background(edge_color));
                }
            }
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
            let (check_w, num_w, pad) = (
                window.pixel_snap(check_w),
                window.pixel_snap(num_w),
                window.pixel_snap(px(NUMBER_PAD)),
            );
            for (i, n) in numbers.iter().enumerate() {
                let Some(label) =
                    n.and_then(|n| CellText::shape(n.to_string(), number_color, window))
                else {
                    continue;
                };
                let right = bounds.left() + check_w + num_w * (i + 1) as f32 - pad;
                label.paint(point(right - label.width, bounds.top()), window, cx);
            }
        };
        canvas(|_, _, _| {}, paint).flex_none().w(width)
    }
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
