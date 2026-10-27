//! Workspace-side handlers of the conflicted-file menu: open and run an action.

use gpui::{Context, Pixels, Point, Window};

use super::conflict_menu_rows::conflict_menu_rows;
use super::state::{ConflictAction, Resolve};
use crate::components::Menu;
use crate::utils::paths::{copy_path, file_url, reveal};
use crate::workspace::Workspace;

impl Workspace {
    pub fn open_conflict_menu(
        &mut self,
        change: gd_git::FileChange,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let menu = Menu::new(position, conflict_menu_rows(), move |ws, action, _, cx| {
            ws.run_conflict_action(action, change.path.clone(), cx)
        });
        self.open_menu(menu, window, cx);
    }

    fn run_conflict_action(&mut self, action: ConflictAction, path: String, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        let root = repo.read(cx).repo.root().to_path_buf();
        let resolve = match action {
            ConflictAction::Ours => Some(Resolve::Ours),
            ConflictAction::Theirs => Some(Resolve::Theirs),
            ConflictAction::Resolved => Some(Resolve::Marked),
            _ => None,
        };
        if let Some(how) = resolve {
            return repo.update(cx, |r, cx| r.resolve_file(path, how, cx));
        }
        match action {
            ConflictAction::OpenEditor => cx.open_url(&file_url(&root.join(&path))),
            ConflictAction::CopyPath => copy_path(&root, &path, cx),
            ConflictAction::Reveal => reveal(&root, &path, cx),
            ConflictAction::Ours | ConflictAction::Theirs | ConflictAction::Resolved => {}
        }
    }
}
