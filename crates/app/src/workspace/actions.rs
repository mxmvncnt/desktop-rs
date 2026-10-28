//! Keyboard / menu action routing for the root view.

use gpui::{Context, InteractiveElement, Stateful, Div, Window};

use super::navigation::{Move, PAGE};
use super::state::{Popover, Workspace};
use crate::actions::*;
use crate::features::merge::MergeMode;
use crate::repo::{RemoteOp, Tab};

/// Register every workspace-level action handler on the root element.
pub(super) fn bind(root: Stateful<Div>, cx: &mut Context<Workspace>) -> Stateful<Div> {
    bind_menu_keys(bind_commands(bind_popovers(root, cx), cx), cx)
}

fn bind_popovers(root: Stateful<Div>, cx: &mut Context<Workspace>) -> Stateful<Div> {
    root.on_action(cx.listener(|this, _: &ToggleRepositoryList, w, cx| {
        this.toggle_popover(Popover::Repositories, w, cx)
    }))
    .on_action(cx.listener(|this, _: &ToggleBranchList, w, cx| this.toggle_popover(Popover::Branches, w, cx)))
    .on_action(cx.listener(|this, _: &NewBranch, w, cx| this.toggle_popover(Popover::NewBranch, w, cx)))
    .on_action(cx.listener(|this, _: &AddRepository, _, cx| this.prompt_add_repository(cx)))
}

fn bind_commands(root: Stateful<Div>, cx: &mut Context<Workspace>) -> Stateful<Div> {
    root.on_action(cx.listener(|this, _: &ShowChanges, _, cx| this.set_tab(Tab::Changes, cx)))
        .on_action(cx.listener(|this, _: &ShowHistory, _, cx| this.set_tab(Tab::History, cx)))
        .on_action(cx.listener(|this, _: &MergeIntoCurrent, w, cx| {
            this.open_merge_dialog(MergeMode::Merge, None, w, cx)
        }))
        .on_action(cx.listener(|this, _: &Fetch, _, cx| this.remote_action(RemoteOp::Fetch, cx)))
        .on_action(cx.listener(|this, _: &Pull, _, cx| this.remote_action(RemoteOp::Pull, cx)))
        .on_action(cx.listener(|this, _: &Push, _, cx| this.remote_action(RemoteOp::Push, cx)))
        .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.toggle_settings(cx)))
        .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
        .on_action(cx.listener(|this, _: &Commit, w, cx| this.commit(w, cx)))
        .on_action(cx.listener(|this, _: &CopyDiffText, _, cx| this.copy_diff_text(cx)))
        .on_action(cx.listener(|this, _: &SelectAllDiffText, _, cx| this.select_all_diff_text(cx)))
        .on_action(cx.listener(|this, _: &PreviewPullRequest, _, cx| this.open_pr_preview(cx)))
        .on_action(cx.listener(|this, _: &StashChanges, w, cx| this.open_stash_create(false, w, cx)))
}

fn bind_menu_keys(root: Stateful<Div>, cx: &mut Context<Workspace>) -> Stateful<Div> {
    root.on_action(cx.listener(|this, _: &MenuNext, _, cx| this.overlay_step(1, cx)))
        .on_action(cx.listener(|this, _: &MenuPrevious, _, cx| this.overlay_step(-1, cx)))
        .on_action(cx.listener(|this, _: &MenuConfirm, w, cx| this.overlay_confirm(w, cx)))
        .on_action(cx.listener(|this, _: &SelectNext, w, cx| this.select_move(Move::By(1), w, cx)))
        .on_action(cx.listener(|this, _: &SelectPrevious, w, cx| this.select_move(Move::By(-1), w, cx)))
        .on_action(cx.listener(|this, _: &SelectFirst, w, cx| this.select_move(Move::First, w, cx)))
        .on_action(cx.listener(|this, _: &SelectLast, w, cx| this.select_move(Move::Last, w, cx)))
        .on_action(cx.listener(|this, _: &SelectPageUp, w, cx| this.select_move(Move::By(-PAGE), w, cx)))
        .on_action(cx.listener(|this, _: &SelectPageDown, w, cx| this.select_move(Move::By(PAGE), w, cx)))
        .on_action(cx.listener(|this, _: &Dismiss, w, cx| this.dismiss(w, cx)))
}

impl Workspace {
    /// Esc: close the top overlay, else leave the force push / pull request preview.
    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.is_empty() {
            if !self.close_force_push_key(cx) {
                self.close_pr_preview_key(cx);
            }
        } else {
            self.close_overlay(window, cx);
        }
    }
}
