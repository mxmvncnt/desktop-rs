//! State of the toolbar popovers: keyboard cursor, inputs and the new-branch base.

use gpui::{App, AppContext, Context, Entity, Pixels, Point, ScrollStrategy, UniformListScrollHandle};
use gpui_elements::editable_text::{EditableTextState, StringStorage};

use crate::workspace::Popover;

pub struct PopoverStore {
    pub selected: usize,
    pub scroll: UniformListScrollHandle,
    pub filter: Entity<EditableTextState>,
    pub name_input: Entity<EditableTextState>,
    pub url_input: Entity<EditableTextState>,
    /// New-branch popover: chosen start point (`None` = current branch).
    pub new_branch_base: Option<String>,
    /// Whether the new-branch base list is expanded.
    pub new_branch_base_open: bool,
    /// Popover the backdrop just dismissed, and where that press landed. The
    /// backdrop doesn't occlude the toolbar, so the same press also clicks
    /// the trigger underneath; this lets the trigger ignore that click
    /// instead of reopening.
    pub dismissed: Option<(Popover, Point<Pixels>)>,
}

impl PopoverStore {
    pub fn new(cx: &mut App) -> Self {
        let input = |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        Self {
            selected: 0,
            scroll: UniformListScrollHandle::new(),
            filter: input(cx),
            name_input: input(cx),
            url_input: input(cx),
            new_branch_base: None,
            new_branch_base_open: false,
            dismissed: None,
        }
    }

    /// Move the picker cursor to row `ix`.
    pub fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected = ix;
        self.scroll.scroll_to_item(ix, ScrollStrategy::Nearest);
        cx.notify();
    }

    /// Keep the cursor on a row of a list with `len` rows.
    pub fn clamp(&mut self, len: usize) -> usize {
        self.selected = self.selected.min(len.saturating_sub(1));
        self.selected
    }

    /// Up/down inside an open popover.
    pub fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.select((self.selected as isize + delta).max(0) as usize, cx)
    }

    pub fn toggle_base_list(&mut self, cx: &mut Context<Self>) {
        self.new_branch_base_open = !self.new_branch_base_open;
        cx.notify();
    }

    /// Pick the start point; `None` goes back to the current branch.
    pub fn set_base(&mut self, base: Option<String>, cx: &mut Context<Self>) {
        self.new_branch_base = base;
        self.new_branch_base_open = false;
        cx.notify();
    }
}
