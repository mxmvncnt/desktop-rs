//! Header of the commit detail pane: summary, body, author, sha, time, file count.

use gd_git::{Commit, LineStats};
use gd_ui::{Icon, ResizeHandle, Theme, icon};
use gpui::{AnyElement, App, DragMoveEvent, Div, Entity, FontWeight, IntoElement, RenderOnce, Window, div, prelude::*, px};

use super::CommitSelection;
use super::message_resize::MessageDrag;
use super::message_resize::MSG_DEFAULT_MAX;
use crate::components::{DiffStat, drag_selection, selectable_line};
use crate::workspace::Workspace;
use crate::utils::text::{authors_list, authors_text, body_without_trailers, full_date};
use crate::utils::text::relative_time;
use crate::utils::text::plural;

#[derive(IntoElement)]
pub struct CommitHeader {
    commit: Commit,
    n_files: usize,
    /// Lines added / deleted by the whole commit.
    stats: LineStats,
    /// Trailing control (the diff options button).
    trailing: AnyElement,
    /// User-set height of the message area (`None` = fit up to the default).
    msg_height: Option<f32>,
    sel: CommitSelection,
    ws: Entity<Workspace>,
}

impl CommitHeader {
    pub fn new(
        commit: Commit,
        n_files: usize,
        stats: LineStats,
        trailing: impl IntoElement,
        msg_height: Option<f32>,
        sel: CommitSelection,
        ws: Entity<Workspace>,
    ) -> Self {
        Self { commit, n_files, stats, trailing: trailing.into_any_element(), msg_height, sel, ws }
    }
}

impl RenderOnce for CommitHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let texts = [self.commit.summary.clone(), body_without_trailers(&self.commit.body)];
        let sel = self.sel.on(&texts);
        let meta = self.meta_line(&theme);
        let (summary, summary_layout) =
            selectable_line(&self.ws, &texts, 0, sel, theme.text, FontWeight::SEMIBOLD, &theme);
        let (body, body_layout) = selectable_line(&self.ws, &texts, 1, sel, theme.text_muted, FontWeight::NORMAL, &theme);
        let has_body = !texts[1].is_empty();
        let message = has_body.then(|| self.message_area(body, &theme));
        let layouts = if has_body { vec![summary_layout, body_layout] } else { vec![summary_layout] };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(6.0))
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(theme.border)
            .on_mouse_move(drag_selection(self.ws.clone(), layouts))
            .child(self.title_line(summary))
            .children(message)
            .child(meta)
    }
}

impl CommitHeader {
    /// Scrollable message with a resize handle below it.
    fn message_area(&self, body: Div, theme: &Theme) -> impl IntoElement + use<> {
        let text = div()
            .id("commit-message")
            .overflow_y_scroll()
            .text_size(px(12.5))
            .text_color(theme.text_muted)
            .when_some(self.msg_height, |d, h| d.h(px(h)))
            .when(self.msg_height.is_none(), |d| d.max_h(px(MSG_DEFAULT_MAX)))
            .child(body);
        let (ws, reset_ws) = (self.ws.clone(), self.ws.clone());
        let shown = self.msg_height.unwrap_or(MSG_DEFAULT_MAX);
        div()
            .flex()
            .flex_col()
            .on_drag_move(move |e: &DragMoveEvent<MessageDrag>, _, cx| {
                let y = e.event.position.y.as_f32();
                ws.update(cx, |this, cx| this.drag_commit_message_to(y, shown, cx))
            })
            .child(text)
            .child(ResizeHandle::bottom_edge("commit-message-handle", MessageDrag).on_reset(move |_, cx| {
                reset_ws.update(cx, |this, cx| this.reset_commit_message_height(cx))
            }))
    }

    fn title_line(self, summary: Div) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap(px(8.0))
            .child(div().flex_1().min_w_0().text_size(px(15.0)).child(summary))
            .child(self.trailing)
    }

    fn meta_line(&self, theme: &Theme) -> impl IntoElement + use<> {
        let c = &self.commit;
        div()
            .flex()
            .items_center()
            .gap(px(14.0))
            .text_size(px(12.0))
            .text_color(theme.text_faint)
            .child(
                div()
                    .id("commit-authors")
                    .tooltip(gd_ui::tooltip(authors_list(c)))
                    .min_w_0()
                    .truncate()
                    .child(authors_text(c)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(icon(Icon::Tag).size(px(12.0)).text_color(theme.text_faint))
                    .font_family(theme.font_mono.clone())
                    .child(c.short_sha.clone()),
            )
            .child(div().id("commit-time").tooltip(gd_ui::tooltip(full_date(c.time))).child(relative_time(c.time)))
            .child(format!("{} changed file{}", self.n_files, plural(self.n_files as u32)))
            .child(DiffStat { added: self.stats.added, deleted: self.stats.deleted })
    }
}
