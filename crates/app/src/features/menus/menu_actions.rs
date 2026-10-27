//! What each Changes context-menu action does.

use gd_git::{FileChange, escape_ignore};
use gpui::{ClipboardItem, Context, Entity, Window};

use super::menu_rows::extension;
use super::state::{ConfirmDialog, MenuAction};
use crate::repo::RepoModel;
use crate::utils::paths::{copy_path, reveal};
use crate::workspace::{Overlay, Workspace};

impl Workspace {
    /// `change` is the right-clicked file (`None` for the header menu).
    pub fn run_menu_action(&mut self, action: MenuAction, change: Option<FileChange>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo.clone() else { return };
        match (action, change) {
            (MenuAction::StashAll, _) => self.open_stash_create(false, window, cx),
            (MenuAction::StashSelected, _) => self.open_stash_create(true, window, cx),
            (MenuAction::DiscardAll, _) => {
                let all = repo.read(cx).changes().to_vec();
                if !all.is_empty() {
                    self.open_overlay(Overlay::Discard(ConfirmDialog::DiscardAll(all)), cx);
                }
            }
            (action, Some(change)) => self.run_file_action(action, change, &repo, cx),
            _ => {}
        }
    }

    fn run_file_action(&mut self, action: MenuAction, c: FileChange, repo: &Entity<RepoModel>, cx: &mut Context<Self>) {
        let root = repo.read(cx).repo.root().to_path_buf();
        match action {
            MenuAction::Discard => self.open_overlay(Overlay::Discard(ConfirmDialog::DiscardFile(c)), cx),
            MenuAction::CopyPath => copy_path(&root, &c.path, cx),
            MenuAction::CopyRelPath => cx.write_to_clipboard(ClipboardItem::new_string(c.path)),
            MenuAction::Reveal => reveal(&root, &c.path, cx),
            MenuAction::Ignore => {
                repo.update(cx, |r, cx| r.ignore(vec![escape_ignore(&c.path)], format!("Ignored {}", c.path), cx))
            }
            MenuAction::IgnoreExt => ignore_extension(&c.path, repo, cx),
            _ => {}
        }
    }
}

fn ignore_extension(path: &str, repo: &Entity<RepoModel>, cx: &mut gpui::App) {
    if let Some(ext) = extension(path) {
        let pattern = format!("*.{}", escape_ignore(ext).trim_start_matches('/'));
        repo.update(cx, |r, cx| r.ignore(vec![pattern.clone()], format!("Ignored {pattern}"), cx))
    }
}
