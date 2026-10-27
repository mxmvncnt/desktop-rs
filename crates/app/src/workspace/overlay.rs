//! The open overlays (`Workspace::overlays`), bottom first = paint order.
//! Opening an overlay replaces whatever was open; the one real stack is the
//! branch context menu opened over the branch picker (`stack_overlay`).
//! Esc pops the top, Enter confirms it and ↑/↓ step it.

use gpui::{Context, Pixels, Point, Window};

use super::state::{Popover, Workspace};
use crate::components::Menu;
use crate::features::branches::BranchDialog;
use crate::features::history::HistoryDialog;
use crate::features::menus::ConfirmDialog;
use crate::features::merge::MergeDialog;
use crate::features::stashes::StashDialog;

#[derive(Clone)]
pub enum Overlay {
    /// Toolbar picker; its state (filter, cursor, ...) stays on `Workspace`.
    Popover(Popover),
    /// Any context menu.
    Menu(Menu),
    /// Diff options popover at the gear button.
    DiffOptions(Point<Pixels>),
    Settings,
    Discard(ConfirmDialog),
    History(HistoryDialog),
    Stash(StashDialog),
    Merge(MergeDialog),
    Branch(BranchDialog),
}

impl Workspace {
    pub fn open_overlay(&mut self, overlay: Overlay, cx: &mut Context<Self>) {
        self.overlays.clear();
        self.stack_overlay(overlay, cx);
    }

    pub fn stack_overlay(&mut self, overlay: Overlay, cx: &mut Context<Self>) {
        self.overlays.push(overlay);
        self.overlay_gen += 1;
        cx.notify();
    }

    /// Esc, a backdrop click or Cancel: close the top overlay.
    pub fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.overlays.pop();
        self.focus(window, cx);
        cx.notify();
    }

    /// The open toolbar picker (it is the bottom of the stack).
    pub fn popover(&self) -> Option<Popover> {
        match self.overlays.first() {
            Some(Overlay::Popover(p)) => Some(*p),
            _ => None,
        }
    }

    /// ↑/↓ while an overlay has the keyboard.
    pub(super) fn overlay_step(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(Overlay::Menu(menu)) = self.overlays.last_mut() {
            menu.step(delta);
            cx.notify();
        }
    }

    /// Enter while an overlay has the keyboard (pickers and settings handle
    /// their own).
    pub(super) fn overlay_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.overlays.last() {
            Some(Overlay::Menu(menu)) => {
                if let Some(run) = menu.confirm() {
                    run(self, window, cx)
                }
            }
            Some(Overlay::Discard(_)) => self.confirm_dialog(cx),
            Some(Overlay::History(_)) => self.hist_confirm(window, cx),
            Some(Overlay::Stash(_)) => self.stash_confirm(cx),
            Some(Overlay::Merge(_)) => self.merge_confirm(window, cx),
            Some(Overlay::Branch(_)) => self.branch_dialog_confirm(window, cx),
            Some(Overlay::Popover(_) | Overlay::DiffOptions(_) | Overlay::Settings) | None => {}
        }
    }
}
