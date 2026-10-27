//! Pull-request preview state (`RepoModel::pr`).

use gd_git::Comparison;
use gpui::UniformListScrollHandle;

use crate::repo::DiffDetail;

#[derive(Default)]
pub struct PrPreviewState {
    /// The main pane shows the preview instead of the tab content.
    pub open: bool,
    /// Chosen base (short branch name); `None` = the repository's default.
    pub base: Option<String>,
    /// The base dropdown is expanded.
    pub base_menu: bool,
    pub loading: bool,
    pub comparison: Option<Comparison>,
    pub error: Option<String>,
    /// Fetch URL of the remote the PR would be opened on.
    pub remote_url: Option<String>,
    /// Files and diff of `comparison` (`base` = its merge base).
    pub detail: DiffDetail,
    pub base_scroll: UniformListScrollHandle,
}
