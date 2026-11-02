//! CI page state: what is shown, the pipelines loaded so far and the per-pipeline detail cache.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gd_forge::{Job, JobId, Log, Pipeline, PipelineDetail, PipelineId, Status};
use gpui::{App, AppContext as _, Entity, EventEmitter, Pixels, Point, Task, UniformListScrollHandle};
use gpui_elements::editable_text::{EditableTextState, StringStorage};

use super::target::{Problem, Target};
use crate::features::diff::text_selection::model::TextSel;
use crate::repo::LoadSlot;

/// Which screen of the page is shown.
/// A text filter of the pipelines list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Branch,
    User,
}

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
    /// Selected text, in (line of `log.lines`, byte offset); `dragging` while the mouse button is down.
    pub sel: Option<TextSel>,
    pub dragging: bool,
    /// The section being opened or collapsed: its rows are in a container under its header whose height animates (`LOG_ANIM`).
    pub anim: Option<LogAnim>,
    /// Counts the toggles: names the animation of a toggle and tells its timer whether it is still the last.
    pub toggles: u64,
}

/// A section being opened or collapsed. The list is laid out with its body hidden for the whole animation, so the lines below are always
/// where they must be drawn: they are pushed by the container of the body, which grows (opening) or shrinks (collapsing).
pub struct LogAnim {
    pub section: usize,
    pub opening: bool,
    pub id: u64,
    pub at: Instant,
    /// The lines of its body, as they show when it is open.
    pub body: Vec<usize>,
}

/// How long a section takes to open or collapse.
pub const LOG_ANIM: Duration = Duration::from_millis(200);

/// A section with more visible rows than this opens and collapses at once: its container would be taller than the window.
pub const MAX_ANIM_ROWS: usize = 40;

/// Empty rows after the last line of a log: the list is virtualized, so its bottom margin is made of rows.
const LOG_BOTTOM_ROWS: usize = 2;

impl LogState {
    /// Rows of the log list: the visible lines and the bottom margin.
    pub fn rows(&self) -> usize {
        self.visible.len() + LOG_BOTTOM_ROWS
    }
}

/// The text inputs of the list filters. They live on the `Workspace`: an `Entity` is made from an `App`, a `CiStore` default has none.
#[derive(Clone)]
pub struct CiInputs {
    pub branch: Entity<EditableTextState>,
    pub user: Entity<EditableTextState>,
    /// Where the dropdown of each opens: below the field, in window coordinates. Set while painting it.
    pub branch_at: Rc<Cell<Point<Pixels>>>,
    pub user_at: Rc<Cell<Point<Pixels>>>,
}

impl CiInputs {
    pub fn new(cx: &mut App) -> Self {
        let mk = |cx: &mut App| cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        Self { branch: mk(cx), user: mk(cx), branch_at: Rc::default(), user_at: Rc::default() }
    }
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
    /// The list filters, empty = none. They follow `CiInputs` (see `set_filters`); `user` only where the account's forge can filter on it.
    pub branch: String,
    pub user: String,
    /// The filter whose dropdown is open.
    pub menu: Option<Field>,
    /// Text was typed in it since it opened: the dropdown lists what contains it.
    pub menu_typed: bool,
    pub pipelines: Vec<Pipeline>,
    /// Last page loaded; a full page means there may be another.
    pub page: u32,
    pub more: bool,
    pub loading: bool,
    /// A refresh (the Refresh button, opening the page) waits for its list: the button is disabled until it is in.
    pub refreshing: bool,
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
    /// Reloads the list once typing in a filter pauses.
    pub(super) filter_task: Option<Task<()>>,
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
            branch: String::new(),
            user: String::new(),
            menu: None,
            menu_typed: false,
            filter_task: None,
            pipelines: Vec::new(),
            page: 1,
            more: false,
            loading: false,
            refreshing: false,
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

    /// The users seen so far, for the suggestions of the user filter.
    pub fn known_users(&self) -> impl Iterator<Item = &gd_forge::User> {
        self.pipelines.iter().filter_map(|p| p.user.as_ref()).chain(self.details.values().filter_map(|d| d.pipeline.user.as_ref()))
    }

    /// A list row wants its stages and they are not loaded, loading or known to fail.
    pub fn needs_detail(&self, id: PipelineId) -> bool {
        !self.details.contains_key(&id) && !self.inflight.contains(&id) && !self.failed.contains(&id)
    }
}
