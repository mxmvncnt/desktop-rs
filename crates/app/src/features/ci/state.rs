//! CI page state: what is shown, the pipelines loaded so far and the per-pipeline detail cache.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use gd_forge::{Pipeline, PipelineDetail, PipelineId, Status};
use gpui::{Task, UniformListScrollHandle};

use super::target::{Problem, Target};
use crate::repo::LoadSlot;

/// Which screen of the page is shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CiView {
    List,
    Pipeline(PipelineId),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PipelineTab {
    Graph,
    Jobs,
}

pub struct CiStore {
    pub open: bool,
    /// Repository the state below belongs to.
    pub(super) root: Option<PathBuf>,
    /// The window is active: polling is paused otherwise.
    pub focused: bool,
    /// `None` while the account is being resolved.
    pub target: Option<Result<Target, Problem>>,
    pub view: CiView,
    pub tab: PipelineTab,
    /// Status tab of the list (`None` = All).
    pub status: Option<Status>,
    pub current_branch: bool,
    pub pipelines: Vec<Pipeline>,
    /// Last page loaded; a full page means there may be another.
    pub page: u32,
    pub more: bool,
    pub loading: bool,
    pub list_error: Option<Problem>,
    /// Details by pipeline: stages for the list dots, the graph and the jobs table. Finished ones never change.
    pub details: HashMap<PipelineId, PipelineDetail>,
    /// Why the open pipeline could not be loaded.
    pub detail_error: Option<Problem>,
    pub list_scroll: UniformListScrollHandle,
    /// Bumped when the repository changes: results of older requests are dropped.
    pub(super) epoch: u64,
    pub(super) inflight: HashSet<PipelineId>,
    /// Details that failed to load; retried on the next poll.
    pub(super) failed: HashSet<PipelineId>,
    pub(super) queue: Vec<PipelineId>,
    pub(super) resolve_load: LoadSlot,
    pub(super) list_load: LoadSlot,
    pub(super) poll: Option<Task<()>>,
}

impl Default for CiStore {
    fn default() -> Self {
        Self {
            open: false,
            root: None,
            focused: true,
            target: None,
            view: CiView::List,
            tab: PipelineTab::Graph,
            status: None,
            current_branch: false,
            pipelines: Vec::new(),
            page: 1,
            more: false,
            loading: false,
            list_error: None,
            detail_error: None,
            details: HashMap::new(),
            list_scroll: UniformListScrollHandle::new(),
            epoch: 0,
            inflight: HashSet::new(),
            failed: HashSet::new(),
            queue: Vec::new(),
            resolve_load: LoadSlot::default(),
            list_load: LoadSlot::default(),
            poll: None,
        }
    }
}

impl CiStore {
    /// The resolved account and repository, if any.
    pub fn ready(&self) -> Option<&Target> {
        self.target.as_ref()?.as_ref().ok()
    }

    /// A list row wants its stages and they are not loaded, loading or known to fail.
    pub fn needs_detail(&self, id: PipelineId) -> bool {
        !self.details.contains_key(&id) && !self.inflight.contains(&id) && !self.failed.contains(&id)
    }
}
