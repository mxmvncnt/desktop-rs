use gd_ui::theme::CONTROL_RADIUS;
use gd_ui::themes::ColorThemes;
use gd_ui::{Button, Icon, Tabs, Theme};
use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    Window, div, px,
};

use super::{ThemeMode, themes_dir};
use crate::utils::paths::file_url;
use crate::workspace::Workspace;

#[derive(IntoElement)]
pub struct AppearancePage {
    pub mode: ThemeMode,
    pub color_theme: String,
    pub ws: Entity<Workspace>,
}

impl AppearancePage {

    fn mode_tabs(&self) -> impl IntoElement {
        let ws = self.ws.clone();
        let selected = ThemeMode::ALL.iter().position(|m| *m == self.mode).unwrap_or(0);
        let tabs = ThemeMode::ALL
            .iter()
            .fold(Tabs::new("settings-theme", selected), |t, m| t.tab(m.label(), None))
            .on_select(move |ix, window, cx| {
                ws.update(cx, |this, cx| this.set_theme_mode(ThemeMode::ALL[ix], window, cx))
            });
        div().w(px(300.0)).child(tabs)
    }

    fn theme_grid(&self, cx: &App) -> impl IntoElement {
        let current = Theme::of(cx).clone();
        let light = current.light;
        let cards = ColorThemes::of(cx).iter().map(|t| {
            let preview = Theme::for_family(&t.id, light, cx);
            let ws = self.ws.clone();
            let id = t.id.clone();
            let selected = t.id == self.color_theme;
            let border = if selected { current.accent } else { current.ink(0.14) };
            let dots = [preview.accent, preview.syntax.indigo, preview.syntax.emerald, preview.syntax.amber, preview.danger];
            div()
                .id(format!("color-theme-{id}"))
                .w(px(122.0))
                .p(px(10.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .rounded(px(CONTROL_RADIUS))
                .border_1()
                .border_color(border)
                .bg(preview.bg)
                .cursor_pointer()
                .on_click(move |_, window, cx| ws.update(cx, |this, cx| this.set_color_theme(&id, window, cx)))
                .child(div().text_size(px(12.0)).text_color(preview.text).child(t.label.clone()))
                .child(
                    div()
                        .flex()
                        .gap(px(5.0))
                        .children(dots.map(|c| div().size(px(10.0)).rounded_full().bg(c))),
                )
        });
        div().flex().flex_wrap().gap(px(10.0)).children(cards)
    }
}

impl RenderOnce for AppearancePage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let caption = |text: &'static str| div().text_size(px(12.0)).text_color(theme.text_faint).child(text);
        let title = |text: &'static str| div().font_weight(gpui::FontWeight::MEDIUM).child(text);
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(title("Mode"))
            .child(self.mode_tabs())
            .child(caption("System follows your operating system's light/dark setting."))
            .child(div().h(px(8.0)))
            .child(title("Color theme"))
            .child(self.theme_grid(cx))
            .child(div().flex().child(
                Button::new("open-themes-folder").icon(Icon::Folder).label("Open themes folder").on_click(|_, _, cx| {
                    let Some(dir) = themes_dir() else { return };
                    match std::fs::create_dir_all(&dir) {
                        Ok(()) => cx.open_url(&file_url(&dir)),
                        Err(e) => log::error!("cannot create {}: {e}", dir.display()),
                    }
                }),
            ))
    }
}
