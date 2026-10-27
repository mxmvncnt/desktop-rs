//! `impl Workspace`: opening, toggling and closing the toolbar popovers.

use gpui::{ClickEvent, Context, Focusable, Window};

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
        let dismissed = self.pickers.update(cx, |p, _| p.dismissed.take());
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
        let store = self.pickers.read(cx);
        let inputs = [store.filter.clone(), store.name_input.clone(), store.url_input.clone()];
        let target = match p {
            Popover::Repositories | Popover::Branches => &inputs[0],
            Popover::NewBranch => &inputs[1],
            Popover::Clone => &inputs[2],
        };
        self.pickers.update(cx, |p, cx| p.set_base(None, cx));
        self.reset_popover_selection(cx);
        for input in &inputs {
            input.update(cx, |s, cx| s.emplace("", cx));
        }
        window.focus(&target.focus_handle(cx), cx);
        cx.notify();
    }

    /// Put the picker cursor on the open repository / checked-out branch while
    /// the filter is empty (a settings index is then also a list row), else on
    /// the first row.
    /// Runs on open and on every input change, including the clear on open.
    pub(crate) fn reset_popover_selection(&mut self, cx: &mut Context<Self>) {
        let unfiltered = self.pickers.read(cx).filter.read(cx).as_str().trim().is_empty();
        let row = match self.popover() {
            Some(Popover::Repositories) if unfiltered => self.settings.selected.unwrap_or(0),
            Some(Popover::Branches) if unfiltered => self.head_branch_row(cx),
            _ => 0,
        };
        self.pickers.update(cx, |p, cx| p.select(row, cx));
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
}

/// The toolbar button that owns a popover.
fn popover_trigger(p: Popover) -> Popover {
    match p {
        Popover::Repositories | Popover::Clone => Popover::Repositories,
        Popover::Branches | Popover::NewBranch => Popover::Branches,
    }
}
