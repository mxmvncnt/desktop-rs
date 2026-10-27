//! Which pane a diff is shown in.

/// Which pane (and `RepoModel::detail`) a diff belongs to: decides line
/// selection and expansion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiffPane {
    /// Working-tree diff: lines can be selected and hunks expanded.
    Changes,
    /// Commit diff: hunks can be expanded.
    Commit,
    /// Stash diff: view only.
    Stash,
    /// Pull-request preview: view only.
    Pr,
}
