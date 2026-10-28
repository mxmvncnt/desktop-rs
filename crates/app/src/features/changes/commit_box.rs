//! The commit box: one message input (first line = summary, rest = description), the amend
//! notice and the commit button.
//!
//! It is its own entity, so typing notifies only this view, not `Workspace`. (gpui-ce still runs
//! `Workspace::render` on the next draw: the window root is not cached and a notified view
//! dirties its ancestors. See ARCHITECTURE.md, "Commit box".)

use std::path::PathBuf;

use gd_git::Commit;
use gd_ui::{Button, Icon, Theme};
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px,
};
use gpui_elements::editable_text::{EditableTextState, StringStorage, TextChanged};

use super::amend_notice::AmendNotice;
use super::state::{CommitButton, SelectionSummary};
use crate::components::text_area;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

const PADDING: f32 = 12.0;
/// The message input grows up to this height, then scrolls.
const MAX_INPUT_HEIGHT: f32 = 200.0;
const SIDEBAR_BORDER: f32 = 1.0;

pub struct CommitBox {
    pub message: Entity<EditableTextState>,
    /// Root of the repository whose last commit is being amended.
    amend: Option<PathBuf>,
    repo: Option<Entity<RepoModel>>,
    /// Placeholder last shown in `message` (see `sync_placeholder`).
    placeholder: Option<String>,
    /// For the sidebar width only.
    ws: WeakEntity<Workspace>,
    _input_sub: Subscription,
    repo_sub: Option<Subscription>,
}

impl CommitBox {
    pub fn new(ws: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        let message = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let _input_sub = cx.subscribe(&message, |_, _, _: &TextChanged, cx| cx.notify());
        Self { message, amend: None, repo: None, placeholder: None, ws, _input_sub, repo_sub: None }
    }

    /// Follow `repo` (re-render and refresh the placeholder when it changes).
    pub fn attach(&mut self, repo: &Entity<RepoModel>, cx: &mut Context<Self>) {
        self.repo = Some(repo.clone());
        self.repo_sub = Some(cx.observe(repo, |this, _, cx| {
            this.sync_placeholder(cx);
            cx.notify();
        }));
    }

    pub fn amending(&self, cx: &gpui::App) -> bool {
        match (&self.amend, &self.repo) {
            (Some(root), Some(r)) => root == r.read(cx).repo.root(),
            _ => false,
        }
    }

    /// (summary, description) of the typed message.
    pub fn text(&self, cx: &gpui::App) -> (String, String) {
        let text = self.message.read(cx).as_str();
        let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
        (first.trim().to_string(), rest.trim().to_string())
    }

    pub fn set_text(&mut self, summary: &str, description: &str, cx: &mut Context<Self>) {
        let text = gd_git::message(summary, description);
        self.message.update(cx, |s, cx| s.emplace(&text, cx));
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.message.update(cx, |s, cx| s.emplace("", cx));
    }

    /// Put `commit`'s message in the box and switch to amend mode.
    pub fn start_amend(&mut self, commit: &Commit, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = &self.repo else { return };
        self.amend = Some(repo.read(cx).repo.root().to_path_buf());
        self.set_text(&commit.summary, &commit.body, cx);
        window.focus(&gpui::Focusable::focus_handle(&self.message, cx), cx);
        cx.notify();
    }

    /// Leave amend mode and empty the box.
    pub fn stop_amend(&mut self, cx: &mut Context<Self>) {
        if self.amend.take().is_some() {
            self.clear(cx);
            cx.notify();
        }
    }

    /// gpui-ce caches the input's layout by text version, not by placeholder, so an empty box
    /// would keep painting the previous placeholder (e.g. the last commit's "Update x").
    /// Re-emplacing "" bumps the version. Runs from the repo observer, never from `render`.
    fn sync_placeholder(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = &self.repo else { return };
        let placeholder = repo.read(cx).default_summary(self.amending(cx));
        if placeholder == self.placeholder {
            return;
        }
        self.placeholder = placeholder;
        if self.message.read(cx).as_str().is_empty() {
            self.clear(cx);
        }
    }
}

impl Render for CommitBox {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let repo = self.repo.clone().expect("commit box is rendered with a repository");
        let amending = self.amending(cx);
        let summary_typed = !self.text(cx).0.is_empty();
        let r = repo.read(cx);
        let b = CommitButton::of(r, SelectionSummary::of(r).included, amending, summary_typed);
        let amend_notice = amending.then(|| AmendNotice { pushed: b.head_pushed, commit_box: cx.entity() });
        let commit = Button::new("commit")
            .primary()
            .full_width()
            .icon(if amending { Icon::Pen } else { Icon::Check })
            .label(b.label.clone())
            .disabled(!b.enabled)
            .on_click(|_, w, cx| w.dispatch_action(Box::new(crate::actions::Commit), cx));
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(8.0))
            .p(px(PADDING))
            .border_t_1()
            .border_color(theme.border)
            .children(amend_notice)
            .child(self.message_input(b.default_summary.as_deref(), cx))
            .child(commit)
    }
}

impl CommitBox {
    fn message_input(&self, default_summary: Option<&str>, cx: &gpui::App) -> impl IntoElement + use<> {
        let placeholder = default_summary.unwrap_or("Commit message (required)");
        // A definite width: with `w_full` gpui-ce also measures the text at max-content
        // width, and its layout cache then paints that unwrapped layout.
        let width = self.ws.read_with(cx, |ws, _| ws.sidebar_width()).unwrap_or_default();
        text_area("message", &self.message, placeholder, cx)
            .w(px(width - SIDEBAR_BORDER - 2.0 * PADDING))
            .min_h(px(64.0))
            .max_h(px(MAX_INPUT_HEIGHT))
            .flex_col()
            .whitespace_normal()
            .overflow_y_scroll()
    }
}

#[cfg(all(test, feature = "snapshot"))]
#[path = "tests/commit_box.test.rs"]
mod tests;
