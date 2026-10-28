//! The "Merge into current branch" dialog: mode tabs, filter, candidate list
//! and preview sentence. The arrows are captured here so the filter input
//! doesn't swallow them.

use gd_git::{Branch, MergePreview};
use gd_ui::{Dialog, Tabs, Theme};
use gpui::{
    App, Context, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, prelude::*,
    UniformListScrollHandle, Window, div, px,
};
use gpui_elements::editable_text::{
    EditableTextState,
    actions::{NavDown, NavUp},
};

use super::merge_candidate_list::{MergeCandidateList, ROW_HEIGHT};
use super::merge_preview::MergePreviewLine;
use super::state::{MergeDialog, MergeMode};
use crate::components::text_field;
use crate::workspace::Workspace;

const LIST_ROWS: usize = 7;

#[derive(IntoElement)]
pub struct MergeDialogView {
    mode: MergeMode,
    ix: usize,
    loading: bool,
    ready: bool,
    preview: Option<MergePreview>,
    current: String,
    candidates: Vec<Branch>,
    filter: Entity<EditableTextState>,
    summary: Entity<EditableTextState>,
    scroll: UniformListScrollHandle,
    ws: Entity<Workspace>,
}

impl MergeDialogView {
    /// `None` without an open repository.
    pub fn open(ws: &Workspace, d: MergeDialog, cx: &mut Context<Workspace>) -> Option<Self> {
        Some(Self {
            mode: d.mode,
            ix: d.ix,
            loading: d.loading,
            ready: ws.merge_ready(),
            preview: d.preview,
            current: ws.repo.as_ref()?.read(cx).branch_name(),
            candidates: ws.merge_candidates(cx),
            filter: ws.merge_ui.filter.clone(),
            summary: ws.merge_ui.summary.clone(),
            scroll: ws.merge_ui.scroll.clone(),
            ws: cx.entity(),
        })
    }
}

impl RenderOnce for MergeDialogView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (ws_ok, ws_cancel) = (self.ws.clone(), self.ws.clone());
        let dialog = Dialog::new("merge-dialog", title(self.mode, &self.current))
            .width(520.0)
            .content(self.content(cx))
            .confirm_label(confirm_label(self.mode))
            .confirm_disabled(!self.ready)
            .on_confirm(move |w, cx| ws_ok.update(cx, |this, cx| this.merge_confirm(w, cx)))
            .on_cancel(move |w, cx| ws_cancel.update(cx, |this, cx| this.close_overlay(w, cx)));
        key_capture(&self.ws).child(dialog)
    }
}

impl MergeDialogView {
    fn content(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx).clone();
        let list = MergeCandidateList {
            branches: self.candidates.clone(),
            selected: self.ix,
            scroll: self.scroll.clone(),
            ws: self.ws.clone(),
        };
        let status = MergePreviewLine {
            mode: self.mode,
            current: self.current.clone(),
            preview: self.preview.clone(),
            loading: self.loading,
            has_candidates: !self.candidates.is_empty(),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .mt(px(6.0))
            .child(self.tabs())
            .child(text_field("merge-filter", &self.filter, "Filter branches", cx))
            .child(
                div()
                    .h(px(ROW_HEIGHT * LIST_ROWS as f32 + 8.0))
                    .px(px(2.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.ink(0.02))
                    .child(list),
            )
            .child(div().min_h(px(36.0)).child(status))
            .when(self.mode == MergeMode::Squash, |d| d.child(self.summary_field(cx)))
    }

    /// Commit summary of the squash commit.
    fn summary_field(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(div().text_size(px(12.0)).text_color(theme.text_muted).child("Squash commit summary"))
            .child(text_field("merge-summary", &self.summary, "Commit summary", cx))
    }

    fn tabs(&self) -> Tabs {
        let ws = self.ws.clone();
        Tabs::new("merge-mode", self.mode as usize)
            .tab("Merge", None)
            .tab("Squash and merge", None)
            .tab("Rebase", None)
            .on_select(move |i, _, cx| {
                let mode = MergeMode::ALL[i];
                ws.update(cx, |this, cx| this.merge_set_mode(mode, cx))
            })
    }
}

/// Full-screen wrapper capturing the arrows before the filter input sees them.
fn key_capture(ws: &Entity<Workspace>) -> gpui::Div {
    let (down, up) = (ws.clone(), ws.clone());
    div()
        .absolute()
        .inset_0()
        .capture_action(move |_: &NavDown, _, cx| {
            cx.stop_propagation();
            down.update(cx, |this, cx| this.merge_step(1, cx))
        })
        .capture_action(move |_: &NavUp, _, cx| {
            cx.stop_propagation();
            up.update(cx, |this, cx| this.merge_step(-1, cx))
        })
}

fn title(mode: MergeMode, current: &str) -> String {
    match mode {
        MergeMode::Merge => format!("Merge into {current}"),
        MergeMode::Squash => format!("Squash and merge into {current}"),
        MergeMode::Rebase => format!("Rebase {current}"),
    }
}

fn confirm_label(mode: MergeMode) -> &'static str {
    match mode {
        MergeMode::Merge => "Create a merge commit",
        MergeMode::Squash => "Squash and merge",
        MergeMode::Rebase => "Rebase",
    }
}
