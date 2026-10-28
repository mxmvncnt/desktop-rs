use gd_ui::Theme;
use gpui::{App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

#[derive(IntoElement)]
pub struct AboutPage;

impl RenderOnce for AboutPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx);
        let row = |label: &'static str, value: &'static str| {
            div()
                .flex()
                .gap(px(12.0))
                .child(div().w(px(100.0)).text_color(theme.text_muted).child(label))
                .child(div().child(value))
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().text_size(px(18.0)).font_weight(gpui::FontWeight::SEMIBOLD).child("Desktop"))
            .child(
                div()
                    .pb(px(10.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_faint)
                    .child("A Rust rewrite of GitHub Desktop."),
            )
            .child(row("Version", env!("CARGO_PKG_VERSION")))
            .child(row("License", env!("CARGO_PKG_LICENSE")))
            .child(row("Built with", "Rust and GPUI-CE"))
            .child(row("Source", "github.com/mxmvncnt/desktop-rs"))
    }
}
