use gd_git::FileChange;

#[derive(Clone)]
pub enum ConfirmDialog {
    DiscardFile(FileChange),
    DiscardAll(Vec<FileChange>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    Discard,
    DiscardAll,
    Ignore,
    IgnoreExt,
    CopyPath,
    CopyRelPath,
    Reveal,
    StashAll,
    StashSelected,
}
