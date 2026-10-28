//! Force push preview state (`RepoModel::force_push`).

use gd_git::ForcePushPreview;

use crate::repo::DiffDetail;

#[derive(Default)]
pub struct ForcePushState {
    /// The main pane shows the preview instead of the tab content.
    pub open: bool,
    pub loading: bool,
    pub preview: Option<ForcePushPreview>,
    pub error: Option<String>,
    /// Files and diff of `preview` (`base` = the remote tip).
    pub detail: DiffDetail,
}
