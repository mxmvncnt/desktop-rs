//! Repository settings → Info: where the repository is and what it points at (read-only).

use gd_git::{RepoInfo, Repository};
use gd_ui::{Button, Theme};
use gpui::{App, ClipboardItem, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};

use crate::utils::paths::file_url;

#[derive(IntoElement)]
pub struct InfoPage {
    pub repo: Repository,
    pub branch: String,
    pub info: Option<RepoInfo>,
    pub error: Option<String>,
}

impl RenderOnce for InfoPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let row = |label: &'static str, value: String, buttons: Vec<Button>| {
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .min_h(px(28.0))
                .child(div().w(px(110.0)).flex_none().text_color(theme.text_muted).child(label))
                .child(div().flex_1().min_w_0().font_family(theme.font_mono.clone()).text_size(px(12.0)).child(SharedString::from(value)))
                .children(buttons)
        };
        let root = self.repo.root().to_path_buf();
        let (copy_path, open_path) = (root.display().to_string(), root.clone());
        let none = || "—".to_string();
        let info = self.info.as_ref();
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(row("Name", self.repo.name(), vec![]))
            .child(row(
                "Path",
                root.display().to_string(),
                vec![
                    Button::new("repo-path-copy").compact().ghost().label("Copy").on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(copy_path.clone()))
                    }),
                    Button::new("repo-path-open").compact().ghost().label("Open folder").on_click(move |_, _, cx| {
                        cx.open_url(&file_url(&open_path))
                    }),
                ],
            ))
            .child(row("Current branch", self.branch, vec![]))
            .child(row("Default remote", info.and_then(|i| i.default_remote.clone()).unwrap_or_else(none), vec![]))
            .child(row("HEAD", info.and_then(|i| i.head.clone()).unwrap_or_else(none), vec![]))
            .child(row("Git directory", self.repo.git_dir().display().to_string(), vec![]))
            .child(row("Size", info.map_or_else(none, |i| format!("{:.1} MiB", i.size_kib as f64 / 1024.0)), vec![]))
            .children(self.error.map(|e| div().pt(px(8.0)).text_size(px(12.0)).text_color(theme.danger).child(e)))
    }
}
