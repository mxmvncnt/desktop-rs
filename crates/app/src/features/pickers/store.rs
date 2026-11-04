//! State of the toolbar popovers: keyboard cursor, inputs and the new-branch base.

use gpui::{App, AppContext, Context, Entity, ScrollStrategy, UniformListScrollHandle, WeakEntity};
use gpui_elements::editable_text::{EditableTextState, StringStorage};

use crate::components::{ComboBox, ComboEntry};
use crate::workspace::Workspace;

pub struct PopoverStore {
    pub selected: usize,
    pub scroll: UniformListScrollHandle,
    pub filter: Entity<EditableTextState>,
    pub name_input: Entity<EditableTextState>,
    pub url_input: Entity<EditableTextState>,
    /// New-branch popover: chosen start point (`None` = current branch).
    pub new_branch_base: Option<String>,
    /// The "Based on" field: empty for the current branch.
    pub base: Entity<ComboBox>,
}

impl PopoverStore {
    pub fn new(ws: WeakEntity<Workspace>, cx: &mut App) -> Self {
        let input =
            |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        Self {
            selected: 0,
            scroll: UniformListScrollHandle::new(),
            filter: input(cx),
            name_input: input(cx),
            url_input: input(cx),
            new_branch_base: None,
            base: cx.new(|cx| {
                ComboBox::new(
                    "new-branch-base",
                    "",
                    move |typed, cx| {
                        let Some(repo) = ws.upgrade().and_then(|ws| ws.read(cx).repo.clone())
                        else {
                            return Vec::new();
                        };
                        let repo = repo.read(cx);
                        let (current, typed) = (repo.branch_name(), typed.to_lowercase());
                        // Every branch that contains the text (ignoring case), the current one first.
                        let mut entries: Vec<ComboEntry> = repo
                            .branches
                            .iter()
                            .filter(|b| b.name.to_lowercase().contains(&typed))
                            .map(|b| ComboEntry {
                                detail: (b.name == current).then(|| "current".to_string()),
                                value: b.name.clone(),
                            })
                            .collect();
                        if let Some(ix) = entries.iter().position(|e| e.detail.is_some()) {
                            entries[..=ix].rotate_right(1);
                        }
                        entries
                    },
                    cx,
                )
            }),
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

    /// Start from `base` (`None` = the current branch) and show it in the field.
    pub fn set_base(&mut self, base: Option<String>, cx: &mut Context<Self>) {
        let shown = base.as_deref().unwrap_or("");
        self.base.update(cx, |c, cx| c.set_value(shown, cx));
        self.new_branch_base = base;
        cx.notify();
    }
}
