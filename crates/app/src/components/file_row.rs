//! One changed file: directory, name and status badge (or a conflict pill).
//! The changes list adds a leading checkbox and a right-click handler.

use gd_git::{FileChange, FileStatus};
use gd_ui::{Icon, Theme, icon, pressable, theme::ROW_HEIGHT};
use gpui::{
    AnyElement, App, ColorExt as _, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::shared::split_path::split_path;

type RightClick = Box<dyn Fn(Point<Pixels>, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct FileRow {
    /// Element id namespace, so lists on screen together never collide.
    id: &'static str,
    ix: usize,
    change: FileChange,
    selected: bool,
    leading: Option<AnyElement>,
    on_click: Box<dyn Fn(&mut Window, &mut App)>,
    on_right_click: Option<RightClick>,
}

impl FileRow {
    pub fn new(
        id: &'static str,
        ix: usize,
        change: FileChange,
        selected: bool,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self { id, ix, change, selected, leading: None, on_click: Box::new(on_click), on_right_click: None }
    }

    pub fn leading(mut self, el: impl IntoElement) -> Self {
        self.leading = Some(el.into_any_element());
        self
    }

    pub fn on_right_click(mut self, f: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static) -> Self {
        self.on_right_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for FileRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (dir, name) = split_path(&self.change.path);
        let on_click = self.on_click;
        let badge = if self.change.status == FileStatus::Conflicted {
            conflict_pill(&theme)
        } else {
            status_badge(self.change.status, &theme)
        };
        let mut row = pressable((self.id, self.ix), self.selected, cx)
            .flex()
            .items_center()
            .gap(px(10.0))
            .h(px(ROW_HEIGHT))
            .w_full()
            .px(px(8.0))
            .rounded(px(6.0))
            .children(self.leading)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(12.5))
                    .child(div().flex_shrink(1.0).min_w_0().truncate().text_color(theme.text_faint).child(dir.to_string()))
                    .child(
                        div()
                            .flex_none()
                            .text_color(if self.selected { theme.text } else { theme.text_muted })
                            .child(name.to_string()),
                    ),
            )
            .child(badge)
            .on_click(move |_, w, cx| on_click(w, cx));
        if let Some(f) = self.on_right_click {
            row = row.on_mouse_down(MouseButton::Right, move |e, w, cx| f(e.position, w, cx));
        }
        row
    }
}

/// Conflict pill: red wash + warning glyph.
fn conflict_pill(theme: &Theme) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(3.0))
        .px(px(6.0))
        .h(px(18.0))
        .rounded_full()
        .bg(theme.danger.opacity(0.14))
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.danger)
        .child(icon(Icon::Warning).size(px(11.0)).text_color(theme.danger))
        .child("Conflict")
        .into_any_element()
}

fn status_badge(status: FileStatus, theme: &Theme) -> AnyElement {
    div()
        .flex_none()
        .w(px(14.0))
        .text_size(px(13.0))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(status_color(status, theme))
        .child(status.badge())
        .into_any_element()
}

fn status_color(status: FileStatus, t: &Theme) -> Hsla {
    match status {
        FileStatus::New | FileStatus::Untracked => t.success,
        FileStatus::Deleted | FileStatus::Conflicted => t.danger,
        FileStatus::Renamed | FileStatus::Copied => t.accent,
        FileStatus::Modified | FileStatus::TypeChanged => t.warning,
    }
}
