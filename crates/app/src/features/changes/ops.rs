//! RepoModel operations of the Changes tab: what is included in the next
//! commit, committing, ignoring, discarding and undoing the last commit.

use std::sync::Arc;

use gd_git::{Diff, FileChange, FileStatus};
use gpui::Context;

use crate::repo::{LastCommit, Op, RepoEvent, RepoModel};
use crate::utils::text::plural;

impl RepoModel {
    pub fn included_paths(&self) -> Vec<String> {
        self.changes()
            .iter()
            .filter(|c| !self.work.excluded.contains(&c.path) && !self.work.partial.contains_key(&c.path))
            .map(|c| c.path.clone())
            .collect()
    }

    /// Name used when the summary is left empty and exactly one file is
    /// selected: "Create x" / "Update x" / "Delete x" / "Rename a to b".
    /// Never while amending or merging.
    pub fn default_summary(&self, amending: bool) -> Option<String> {
        if amending || self.merging() {
            return None;
        }
        let mut selected = self.changes().iter().filter(|c| !self.work.excluded.contains(&c.path));
        let c = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let name = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
        Some(match (c.status, &c.old_path) {
            (FileStatus::New | FileStatus::Untracked, _) => format!("Create {}", name(&c.path)),
            (FileStatus::Deleted, _) => format!("Delete {}", name(&c.path)),
            (FileStatus::Renamed, Some(old)) => format!("Rename {} to {}", name(old), name(&c.path)),
            _ => format!("Update {}", name(&c.path)),
        })
    }

    /// File checkbox: a partially selected file becomes fully included.
    pub fn toggle_included(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.work.partial.remove(path).is_some() {
            self.work.excluded.remove(path);
        } else if !self.work.excluded.remove(path) {
            self.work.excluded.insert(path.to_string());
        }
        cx.notify();
    }

    pub fn set_all_included(&mut self, included: bool, cx: &mut Context<Self>) {
        self.work.partial.clear();
        if included {
            self.work.excluded.clear();
        } else {
            self.work.excluded = self.changes().iter().map(|c| c.path.clone()).collect();
        }
        cx.notify();
    }

    pub fn commit(&mut self, summary: String, description: String, cx: &mut Context<Self>) {
        let paths = self.included_paths();
        let partial = self.partial_files();
        let n = paths.len() + partial.len();
        let branch = self.branch_name();
        if self.merging() {
            return self.commit_merge(summary, description, cx);
        }
        let message = (summary.clone(), description.clone());
        self.run(
            Op { committing: true, ..Op::default() },
            move |r| r.commit_partial(&summary, &description, &paths, &partial),
            move |this, _sha, _| {
                this.last_commit = Some(LastCommit {
                    summary: message.0,
                    description: message.1,
                    branch,
                    time: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                });
                this.notice = Some(format!(
                    "Committed {n} file{}",
                    plural(n as u32)
                ));
                // Keep the selection: the status refresh moves it to the next
                // file and the old diff stays up until the new one is loaded,
                // so there is no empty frame in between.
                // Row indices refer to diffs that no longer exist.
                this.work.partial.clear();
            },
            cx,
        );
    }

    /// Append gitignore `patterns` to `.gitignore`, then refresh.
    pub fn ignore(&mut self, patterns: Vec<String>, notice: String, cx: &mut Context<Self>) {
        self.run(
            Op::default(),
            move |r| r.ignore(&patterns),
            move |this, _, _| this.notice = Some(notice),
            cx,
        );
    }

    pub fn discard(&mut self, changes: Vec<FileChange>, cx: &mut Context<Self>) {
        self.run(Op::default(), move |r| r.discard(&changes), |_, _, _| {}, cx);
    }

    /// Revert the changed `rows` of `diff` in the working-tree file `path`.
    pub fn discard_rows(&mut self, path: String, diff: Arc<Diff>, rows: Vec<usize>, cx: &mut Context<Self>) {
        self.run(Op::default(), move |r| r.discard_rows(&path, &diff, &rows), |_, _, _| {}, cx);
    }

    /// Soft-reset HEAD~1 and hand the message back via [`RepoEvent::CommitUndone`].
    pub fn undo_last_commit(&mut self, cx: &mut Context<Self>) {
        let last = self.last_commit.take();
        self.run(
            Op::default(),
            |r| r.undo_last_commit(),
            move |this, _, cx| {
                this.reset_history_selection();
                if let Some(c) = last {
                    cx.emit(RepoEvent::CommitUndone {
                        summary: c.summary,
                        description: c.description,
                    });
                }
            },
            cx,
        );
    }
}
