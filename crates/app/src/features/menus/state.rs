use std::sync::Arc;

use gd_git::{Diff, FileChange};

#[derive(Clone)]
pub enum ConfirmDialog {
    File(FileChange),
    All(Vec<FileChange>),
    /// Changed `rows` of the working diff of `path`: one line or a whole chunk.
    Rows { path: String, diff: Arc<Diff>, rows: Vec<usize> },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    Discard,
    DiscardAll,
    DiscardRows,
    Ignore,
    IgnoreExt,
    CopyPath,
    CopyRelPath,
    Reveal,
    StashAll,
    StashSelected,
}
