//! Unified / Split tabs plus the wrap and hide-whitespace toggles, shared by
//! the diff options popover (`sectioned`) and the settings page.

use gd_ui::{CheckRow, Tabs, Theme};
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct DiffDisplayOptions {
    /// Popover layout: titled sections and a separator.
    pub sectioned: bool,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for DiffDisplayOptions {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let settings = &self.ws.read(cx).settings;
        let (split, wrap, hidden) = (settings.diff_split, settings.diff_wrap, settings.diff_ignore_whitespace);
        let (ws_tabs, ws_wrap, ws_hide) = (self.ws.clone(), self.ws.clone(), self.ws);
        let tabs = div().px(px(2.0)).pb(px(6.0)).child(
            Tabs::new("diff-display", usize::from(split))
                .tab("Unified", None)
                .tab("Split", None)
                .on_select(move |ix, _, cx| ws_tabs.update(cx, |this, cx| this.set_diff_split(ix == 1, cx))),
        );
        let wrap_row = CheckRow::new("diff-opt-wrap", "Wrap long lines", wrap, move |_, cx| {
            ws_wrap.update(cx, |this, cx| this.set_diff_wrap(!wrap, cx))
        });
        let hide_row = CheckRow::new("diff-opt-ws", "Hide whitespace changes", hidden, move |_, cx| {
            ws_hide.update(cx, |this, cx| this.set_ignore_whitespace(!hidden, cx))
        });
        if self.sectioned {
            div()
                .flex()
                .flex_col()
                .child(section_label("Diff display", theme))
                .child(tabs)
                .child(wrap_row)
                .child(div().h(px(1.0)).mx(px(2.0)).my(px(4.0)).bg(theme.border))
                .child(section_label("Whitespace", theme))
                .child(hide_row)
        } else {
            div().flex().flex_col().gap(px(6.0)).child(tabs).child(wrap_row).child(hide_row)
        }
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
