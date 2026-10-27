//! The diff options popover: Unified / Split, "Wrap long lines", "Hide
//! whitespace changes". A `deferred` overlay like `ContextMenu` (closes on
//! backdrop click/Esc; counted in `overlay_open`).

use gd_ui::{Popover, Tabs, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, Pixels, Point, RenderOnce, Styled, Window, div, px};

use super::check_option_row::CheckOptionRow;
use crate::workspace::Workspace;

const MENU_W: f32 = 264.0;

#[derive(IntoElement)]
pub struct DiffOptionsMenu {
    pos: Point<Pixels>,
    /// Popover open counter: restarts the entrance animation per open.
    generation: usize,
    split: bool,
    wrap: bool,
    hide_whitespace: bool,
    ws: Entity<Workspace>,
}

impl DiffOptionsMenu {
    /// `None` while the popover is closed.
    pub fn new(ws: &Workspace, entity: Entity<Workspace>) -> Option<Self> {
        Some(Self {
            pos: ws.diff_view.menu?,
            generation: ws.diff_view.menu_gen,
            split: ws.settings.diff_split,
            wrap: ws.settings.diff_wrap,
            hide_whitespace: ws.settings.diff_ignore_whitespace,
            ws: entity,
        })
    }
}

impl RenderOnce for DiffOptionsMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ws = self.ws.clone();
        Popover::new(("diff-options-in", self.generation), self.origin(window), MENU_W, self.card(cx))
            .on_dismiss(move |_, _, cx| ws.update(cx, |this, cx| this.close_diff_options(cx)))
    }
}

impl DiffOptionsMenu {
    /// Top-left of the card: under the gear button, kept inside the window.
    fn origin(&self, window: &Window) -> Point<Pixels> {
        let inset = window.client_inset().unwrap_or(px(0.0));
        let view = window.viewport_size();
        let x = (self.pos.x - inset - px(MENU_W - 24.0))
            .min(view.width - inset * 2.0 - px(MENU_W + 8.0))
            .max(px(8.0));
        Point { x, y: self.pos.y - inset + px(18.0) }
    }

    fn card(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx);
        let (wrap, hidden) = (self.wrap, self.hide_whitespace);
        let ws_tabs = self.ws.clone();
        div()
            .p(px(6.0))
            .flex()
            .flex_col()
            .child(section_label("Diff display", theme))
            .child(
                div().px(px(2.0)).pb(px(6.0)).child(
                    Tabs::new("diff-display", usize::from(self.split))
                        .tab("Unified", None)
                        .tab("Split", None)
                        .on_select(move |ix, _, cx| ws_tabs.update(cx, |this, cx| this.set_diff_split(ix == 1, cx))),
                ),
            )
            .child(CheckOptionRow::new("diff-opt-wrap", "Wrap long lines", wrap, self.ws.clone(), move |this, cx| {
                this.set_diff_wrap(!wrap, cx)
            }))
            .child(div().h(px(1.0)).mx(px(2.0)).my(px(4.0)).bg(theme.border))
            .child(section_label("Whitespace", theme))
            .child(CheckOptionRow::new(
                "diff-opt-ws",
                "Hide whitespace changes",
                hidden,
                self.ws.clone(),
                move |this, cx| this.set_ignore_whitespace(!hidden, cx),
            ))
    }
}

fn section_label(text: &'static str, theme: &Theme) -> impl IntoElement {
    div()
        .px(px(8.0))
        .pt(px(4.0))
        .pb(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text_faint)
        .child(text)
}
