//! Force push preview state (`RepoModel::force_push`).

use gd_git::ForcePushPreview;

use crate::repo::DiffDetail;

#[derive(Default)]
pub struct ForcePushState {
    /// The modal is open (mirrors `Overlay::ForcePush`; refreshes reload while it is).
    pub open: bool,
    pub loading: bool,
    pub preview: Option<ForcePushPreview>,
    pub error: Option<String>,
    /// Files and diff of `preview` (`base` = the remote tip).
    pub detail: DiffDetail,
}
