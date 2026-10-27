//! `impl Workspace`: opening, toggling and closing the toolbar popovers.

use gpui::{ClickEvent, Context, Focusable, ScrollStrategy, Window};

use crate::workspace::{Overlay, Popover, Workspace};

impl Workspace {
    /// Toolbar trigger click: toggles `p`, unless this press is the one that
    /// just closed it through the backdrop.
    pub(crate) fn click_popover_trigger(
        &mut self,
        p: Popover,
        ev: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dismissed = self.popover_dismissed.take();
        if let (Some((was, at)), ClickEvent::Mouse(m)) = (dismissed, ev)
            && popover_trigger(was) == p
            && at == m.down.position
        {
            return;
        }
        self.toggle_popover(p, window, cx)
    }

    pub fn toggle_popover(&mut self, p: Popover, window: &mut Window, cx: &mut Context<Self>) {
        if self.popover() == Some(p) {
            self.close_overlay(window, cx);
            return;
        }
        if matches!(p, Popover::Branches | Popover::NewBranch) && self.repo.is_none() {
            return;
        }
        self.open_overlay(Overlay::Popover(p), cx);
        self.new_branch_base = None;
        self.new_branch_base_open = false;
        self.reset_popover_selection(cx);
        for input in [&self.filter, &self.name_input, &self.url_input] {
            input.update(cx, |s, cx| s.emplace("", cx));
        }
        let target = match p {
            Popover::Repositories | Popover::Branches => &self.filter,
            Popover::NewBranch => &self.name_input,
            Popover::Clone => &self.url_input,
        };
        window.focus(&target.focus_handle(cx), cx);
        cx.notify();
    }

    /// Put the picker cursor on the open repository / checked-out branch while
    /// the filter is empty (a settings index is then also a list row), else on
    /// the first row.
    /// Runs on open and on every input change, including the clear on open.
    pub(crate) fn reset_popover_selection(&mut self, cx: &mut Context<Self>) {
        let unfiltered = self.filter.read(cx).as_str().trim().is_empty();
        self.popover_selected = match self.popover() {
            Some(Popover::Repositories) if unfiltered => self.settings.selected.unwrap_or(0),
            Some(Popover::Branches) if unfiltered => self.head_branch_row(cx),
            _ => 0,
        };
        self.popover_scroll
            .scroll_to_item(self.popover_selected, ScrollStrategy::Nearest);
    }

    /// Row of the checked-out branch in the unfiltered branch list.
    fn head_branch_row(&self, cx: &Context<Self>) -> usize {
        let Some(repo) = &self.repo else { return 0 };
        let r = repo.read(cx);
        self.filtered_branches(cx)
            .iter()
            .position(|&i| r.branches.get(i).is_some_and(|b| b.is_head))
            .unwrap_or(0)
    }

    /// Up/down inside an open popover.
    pub(crate) fn step_popover_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.popover_selected = (self.popover_selected as isize + delta).max(0) as usize;
        self.popover_scroll
            .scroll_to_item(self.popover_selected, ScrollStrategy::Nearest);
        cx.notify();
    }
}

/// The toolbar button that owns a popover.
fn popover_trigger(p: Popover) -> Popover {
    match p {
        Popover::Repositories | Popover::Clone => Popover::Repositories,
        Popover::Branches | Popover::NewBranch => Popover::Branches,
    }
}
