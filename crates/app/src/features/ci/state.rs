//! CI page state: what is shown, the pipelines loaded so far and the per-pipeline detail cache.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use gd_forge::{Job, JobId, Log, Pipeline, PipelineDetail, PipelineId, Status};
use gpui::{EventEmitter, Task, UniformListScrollHandle};

use super::target::{Problem, Target};
use crate::repo::LoadSlot;

/// Which screen of the page is shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CiView {
    List,
    Pipeline(PipelineId),
    Job(JobId),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PipelineTab {
    Graph,
    Jobs,
}

/// The log of the open job. `visible` are the indices of `log.lines` not hidden by a collapsed section.
#[derive(Default)]
pub struct LogState {
    pub log: Option<Arc<Log>>,
    pub visible: Vec<usize>,
    /// Sections the user flipped: one is collapsed when `Section::collapsed` differs from this.
    pub toggled: HashSet<usize>,
    /// Keep the view at the end while the job runs.
    pub follow: bool,
    pub loading: bool,
    pub error: Option<Problem>,
    pub scroll: UniformListScrollHandle,
}

/// Raised to the `Workspace`, which owns the notice banner.
pub enum CiEvent {
    Notice(String),
    Error(String),
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
    /// Screens left with Back, most recent last; the Forward button walks them again.
    pub(super) forward: Vec<CiView>,
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
    pub log: LogState,
    /// A retry / cancel / play request is running.
    pub acting: bool,
    pub list_scroll: UniformListScrollHandle,
    /// Bumped when the repository changes: results of older requests are dropped.
    pub(super) epoch: u64,
    pub(super) inflight: HashSet<PipelineId>,
    /// Details that failed to load; retried on the next poll.
    pub(super) failed: HashSet<PipelineId>,
    pub(super) queue: Vec<PipelineId>,
    pub(super) resolve_load: LoadSlot,
    pub(super) list_load: LoadSlot,
    pub(super) log_load: LoadSlot,
    pub(super) poll: Option<Task<()>>,
}

impl EventEmitter<CiEvent> for CiStore {}

impl Default for CiStore {
    fn default() -> Self {
        Self {
            open: false,
            root: None,
            focused: true,
            target: None,
            view: CiView::List,
            forward: Vec::new(),
            tab: PipelineTab::Graph,
            status: None,
            current_branch: false,
            pipelines: Vec::new(),
            page: 1,
            more: false,
            loading: false,
            list_error: None,
            detail_error: None,
            log: LogState::default(),
            acting: false,
            details: HashMap::new(),
            list_scroll: UniformListScrollHandle::new(),
            epoch: 0,
            inflight: HashSet::new(),
            failed: HashSet::new(),
            queue: Vec::new(),
            resolve_load: LoadSlot::default(),
            list_load: LoadSlot::default(),
            log_load: LoadSlot::default(),
            poll: None,
        }
    }
}

impl CiStore {
    /// The resolved account and repository, if any.
    pub fn ready(&self) -> Option<&Target> {
        self.target.as_ref()?.as_ref().ok()
    }

    pub fn job(&self, id: JobId) -> Option<&Job> {
        self.details.get(&id.pipeline)?.stages.iter().flat_map(|s| &s.jobs).find(|j| j.id == id)
    }

    /// A list row wants its stages and they are not loaded, loading or known to fail.
    pub fn needs_detail(&self, id: PipelineId) -> bool {
        !self.details.contains_key(&id) && !self.inflight.contains(&id) && !self.failed.contains(&id)
    }
}
