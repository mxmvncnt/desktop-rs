//! Rows of the stash right-click menu.

use gd_ui::Icon;

use crate::components::{MenuRow, RowItem};
use super::state::StashAction;

pub fn stash_menu_rows() -> Vec<MenuRow<StashAction>> {
    vec![
        MenuRow::item(StashAction::Apply, "Apply (keep stash)", Icon::ArchiveUp),
        MenuRow::item(StashAction::Pop, "Pop (apply and remove)", Icon::Restart),
        MenuRow::Separator,
        MenuRow::item(StashAction::Rename, "Rename…", Icon::Pen),
        MenuRow::item(StashAction::Copy, "Copy message", Icon::Copy),
        MenuRow::Separator,
        MenuRow::Item(RowItem::new(StashAction::Drop, "Drop…", Icon::Trash).danger()),
    ]
}
