//! Changes tab of the sidebar: select-all header, file list, stashes, undo
//! strip and the commit box.

use gd_ui::divider;
use gpui::{
    App, Context, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
};

use super::changes_header::ChangesHeader;
use super::changes_list::ChangesList;
use super::commit_box::CommitBox;
use super::state::SelectionSummary;
use super::undo_strip::UndoStrip;
use crate::features::stashes::StashPanel;
use crate::repo::{LastCommit, RepoModel};
use crate::workspace::{NavList, Workspace, nav_focus};

#[derive(IntoElement)]
pub struct ChangesPanel {
    header: ChangesHeader,
    list: ChangesList,
    ws: Entity<Workspace>,
    stashes: Option<StashPanel>,
    undo: Option<UndoStrip>,
    commit_box: Entity<CommitBox>,
}

impl ChangesPanel {
    pub fn new(ws: &Workspace, repo: &Entity<RepoModel>, cx: &mut Context<Workspace>) -> Self {
        let r = repo.read(cx);
        let selection = SelectionSummary::of(r);
        let amending = ws.commit_box.read(cx).amending(cx);
        let last: Option<LastCommit> = r.undoable_commit().cloned().filter(|_| !amending);
        let ws_entity = cx.entity();
        Self {
            header: ChangesHeader { selection, repo: repo.clone(), ws: ws_entity.clone() },
            list: ChangesList {
                count: selection.total,
                repo: repo.clone(),
                ws: ws_entity.clone(),
                scroll: ws.changes_scroll.clone(),
            },
            ws: ws_entity.clone(),
            stashes: StashPanel::new(repo, ws_entity.clone(), cx),
            undo: last.map(|c| UndoStrip { commit: c, ws: ws_entity.clone() }),
            commit_box: ws.commit_box.clone(),
        }
    }
}

impl RenderOnce for ChangesPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(self.header)
            .child(divider(cx))
            .child(nav_focus(div(), NavList::Changes, self.ws).flex().flex_col().flex_1().min_h_0().child(self.list))
            .children(self.stashes)
            .children(self.undo)
            .child(self.commit_box)
    }
}

