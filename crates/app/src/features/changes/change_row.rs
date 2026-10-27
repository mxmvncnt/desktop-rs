//! One row of the changes list. Hot (virtualized): a plain fn, not a component.

use gd_git::FileChange;
use gd_ui::Checkbox;
use gpui::{AnyElement, App, Entity, IntoElement};

use crate::components::FileRow;
use crate::repo::RepoModel;
use crate::workspace::Workspace;

pub(super) fn change_row(
    ix: usize,
    change: FileChange,
    repo: &Entity<RepoModel>,
    ws: &Entity<Workspace>,
    cx: &mut App,
) -> AnyElement {
    let r = repo.read(cx);
    let selected = r.selected_change == Some(ix);
    let check = r.file_check_state(&change.path);
    let (repo_click, repo_check, ws_row) = (repo.clone(), repo.clone(), ws.clone());
    let path = change.path.clone();
    FileRow::new("change", ix, change, selected, move |_, cx| repo_click.update(cx, |r, cx| r.select_change(ix, cx)))
        .leading(Checkbox::new(("check", ix), check).on_click(move |_, _, cx| {
            repo_check.update(cx, |r, cx| r.toggle_included(&path, cx))
        }))
        .on_right_click(move |pos, w, cx| ws_row.update(cx, |this, cx| this.open_file_menu(ix, pos, w, cx)))
        .into_any_element()
}
