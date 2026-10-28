//! The force push preview modal: header, file list and read-only diff. An
//! `impl Workspace` method because the diff renderer owns scroll state (`&mut Workspace`).

use std::rc::Rc;

use gd_ui::{Callback, Theme, modal, motion, overlay_card};
use gpui::{AnyElement, Context, IntoElement, Window, div, prelude::*, px, relative};

use super::force_push_header::ForcePushHeader;
use crate::features::diff::DiffPane;
use crate::repo::RemoteOp;
use crate::workspace::{Overlay, Workspace};

impl Workspace {
    /// Toolbar button / Push shortcut on a rewritten branch: open the preview and load it.
    pub(crate) fn open_force_push(&mut self, cx: &mut Context<Self>) {
        self.open_overlay(Overlay::ForcePush, cx);
        self.with_repo(cx, |r, cx| r.load_force_push(cx));
    }

    /// The Force push button: close the preview, then push.
    pub(crate) fn confirm_force_push(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_overlay(window, cx);
        self.with_repo(cx, |r, cx| r.remote(RemoteOp::ForcePush, cx));
    }

    /// Lets the model know whether the modal is open (it reloads on refresh while it is).
    pub(crate) fn sync_force_push_open(&mut self, cx: &mut Context<Self>) {
        let open = self.overlays.iter().any(|o| matches!(o, Overlay::ForcePush));
        self.with_repo(cx, |r, _| r.force_push.open = open);
    }

    pub(crate) fn render_force_push(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(repo) = self.repo.clone() else { return div().into_any_element() };
        let theme = Theme::of(cx).clone();
        let ws = cx.entity();
        let header = ForcePushHeader { repo: repo.clone(), ws: ws.clone() };
        let r = repo.read(cx);
        let text = match (&r.force_push.preview, r.force_push.loading) {
            (Some(p), _) if !p.files.is_empty() => None,
            (Some(_), _) => Some("The file contents are identical: only the commit history changes."),
            (None, true) => Some("Comparing with the remote…"),
            (None, false) => Some("Nothing to compare with the remote."),
        };
        let body = match text {
            None => self.render_file_diff_pane(header, &repo, DiffPane::ForcePush, cx),
            Some(text) => div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(header)
                .child(div().flex().flex_1().items_center().justify_center().text_color(theme.text_muted).child(text))
                .into_any_element(),
        };
        let card = overlay_card(cx)
            .id("force-push-card")
            .occlude()
            .overflow_hidden()
            .flex()
            .w(relative(0.9))
            .max_w(px(1100.0))
            .h(relative(0.88))
            .max_h(px(760.0))
            .child(body);
        let instant = motion::handoff(&"force-push-in".into(), cx);
        let on_cancel: Callback = Rc::new(move |w, cx| ws.update(cx, |this, cx| this.close_overlay(w, cx)));
        modal(card, Some(on_cancel), instant).into_any_element()
    }
}
