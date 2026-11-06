//! `impl CiStore`: resolving the account, loading pipelines and details, and the requests behind the buttons.

use std::collections::HashSet;

use std::sync::Arc;
use std::time::{Duration, Instant};

use gd_forge::{
    JobId, Log, PipelineDetail, PipelineFilter, PipelineId, parse_log, stages_from_workflow,
};
use gd_git::Repository;
use gpui::{AppContext, Context, ScrollStrategy};

use super::state::{
    CiStore, CiView, Field, LOG_ANIM, LogAnim, LogState, MAX_ANIM_ROWS, PipelineTab,
};
use crate::features::accounts::Target;
use crate::features::accounts::{Account, RepoAccounts};
use crate::repo::spawn_load;

/// The forges answer 20 pipelines per page.
const PAGE_SIZE: usize = 20;
/// Detail requests for the list's stage dots running at once.
const DETAIL_CONCURRENCY: usize = 4;

/// How long typing in a filter pauses before the list reloads.
const FILTER_DEBOUNCE: Duration = Duration::from_millis(350);
/// Where Gitea, Forgejo and GitHub keep workflow files; Forgejo reports a bare file name.
const WORKFLOW_DIRS: [&str; 3] = [
    ".forgejo/workflows",
    ".gitea/workflows",
    ".github/workflows",
];

impl CiStore {
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.poll = None;
        cx.notify();
    }

    /// Esc, the ← button and the mouse Back button: one screen back; from the list, close the page.
    pub fn back(&mut self, cx: &mut Context<Self>) {
        match self.view {
            CiView::List => self.close(cx),
            CiView::Pipeline(_) => {
                self.forward.push(self.view);
                self.view = CiView::List;
            }
            CiView::Job(job) => {
                self.forward.push(self.view);
                self.log_load.cancel();
                self.view = CiView::Pipeline(job.pipeline);
            }
        }
        cx.notify();
    }

    /// The mouse Forward button: the screen Back left last.
    pub fn forward(&mut self, cx: &mut Context<Self>) {
        match self.forward.pop() {
            Some(CiView::Pipeline(id)) => self.show_pipeline(id, cx),
            Some(CiView::Job(id)) => self.show_job(id, cx),
            Some(CiView::List) | None => {}
        }
    }

    /// The "Pipelines" crumb: straight to the list from any screen; Forward comes back to the screen it leaves.
    pub fn show_list(&mut self, cx: &mut Context<Self>) {
        if self.view != CiView::List {
            self.forward.push(self.view);
        }
        self.log_load.cancel();
        self.view = CiView::List;
        cx.notify();
    }

    pub fn open_pipeline(&mut self, id: PipelineId, cx: &mut Context<Self>) {
        self.forward.clear();
        self.show_pipeline(id, cx);
    }

    pub fn open_job(&mut self, id: JobId, cx: &mut Context<Self>) {
        self.forward.clear();
        self.show_job(id, cx);
    }

    fn show_pipeline(&mut self, id: PipelineId, cx: &mut Context<Self>) {
        self.view = CiView::Pipeline(id);
        self.tab = PipelineTab::Graph;
        self.detail_error = None;
        self.fetch_detail(id, cx);
        cx.notify();
    }

    fn show_job(&mut self, id: JobId, cx: &mut Context<Self>) {
        self.view = CiView::Job(id);
        self.log = LogState {
            follow: self.job(id).is_some_and(|j| !j.status.is_finished()),
            ..LogState::default()
        };
        self.load_log(id, cx);
        cx.notify();
    }

    /// Fetch and parse the log of `id` (again, while the job runs).
    pub(super) fn load_log(&mut self, id: JobId, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else {
            return;
        };
        self.log.loading = self.log.log.is_none();
        spawn_load(
            self,
            |s| &mut s.log_load,
            move || {
                let raw = target.client()?.job_log(&target.repo, &id)?;
                Ok::<_, gd_forge::Error>(Arc::new(parse_log(&raw)))
            },
            |this, res, cx| {
                this.log.loading = false;
                match res {
                    Ok(log) => {
                        this.log.error = None;
                        this.log.visible = visible_lines(&log, &this.log.toggled);
                        this.log.log = Some(log);
                        if this.log.follow {
                            this.log
                                .scroll
                                .scroll_to_item(this.log.rows() - 1, ScrollStrategy::Bottom);
                        }
                    }
                    Err(e) => this.log.error = Some(e.into()),
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    /// Collapse or expand the section whose header is line `section`'s header.
    pub fn toggle_section(&mut self, section: usize, cx: &mut Context<Self>) {
        let Some(log) = self.log.log.clone() else {
            return;
        };
        let opening = log.sections[section].collapsed != self.log.toggled.contains(&section);
        let before = visible_lines(&log, &self.log.toggled);
        if !self.log.toggled.remove(&section) {
            self.log.toggled.insert(section);
        }
        let after = visible_lines(&log, &self.log.toggled);
        // The rows of the body as they show when the section is open.
        let s = &log.sections[section];
        let body: Vec<usize> = (if opening { &after } else { &before })
            .iter()
            .copied()
            .filter(|l| (s.header + 1..s.end).contains(l))
            .collect();
        self.log.toggles += 1;
        let id = self.log.toggles;
        if body.is_empty() || body.len() > MAX_ANIM_ROWS {
            self.log.visible = after;
            self.log.anim = None;
        } else if opening {
            // The list keeps its body hidden while the container grows, and gets it when the animation is over.
            self.log.visible = before;
            self.log.anim = Some(LogAnim {
                section,
                opening,
                id,
                at: Instant::now(),
                body,
            });
            // The animation starts at the first paint, a frame after the toggle.
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(LOG_ANIM + Duration::from_millis(50))
                    .await;
                this.update(cx, |s, cx| {
                    if s.log.anim.as_ref().is_some_and(|a| a.id == id)
                        && let Some(log) = &s.log.log
                    {
                        s.log.visible = visible_lines(log, &s.log.toggled);
                        s.log.anim = None;
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        } else {
            // The list loses the body at once; the container of its rows shrinks over where they were.
            self.log.visible = after;
            self.log.anim = Some(LogAnim {
                section,
                opening,
                id,
                at: Instant::now(),
                body,
            });
        }
        cx.notify();
    }

    /// Open the page for `repo` (or refresh it): another repository starts from scratch, the same one keeps its cache.
    pub fn show(
        &mut self,
        repo: Repository,
        branch: Option<String>,
        accounts: Vec<Account>,
        picks: RepoAccounts,
        cx: &mut Context<Self>,
    ) {
        if self.root.as_deref() != Some(repo.root()) {
            *self = Self {
                epoch: self.epoch + 1,
                focused: self.focused,
                poll: self.poll.take(),
                root: Some(repo.root().to_path_buf()),
                ..Self::default()
            };
        }
        self.open = true;
        self.refreshing = true;
        self.start_polling(cx);
        spawn_load(
            self,
            |s| &mut s.resolve_load,
            move || Target::resolve(&repo, branch, &accounts, &picks),
            |this, target, cx| {
                let ready = target.is_ok();
                this.target = Some(target);
                if ready {
                    this.load_list(false, cx);
                } else {
                    this.refreshing = false;
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    /// Page 1 merged into what is loaded (polling), or the next page (`more`).
    pub fn load_list(&mut self, more: bool, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else {
            return;
        };
        if self.loading {
            return;
        }
        let non_empty = |text: &String| (!text.is_empty()).then(|| text.clone());
        let filter = PipelineFilter {
            ref_name: non_empty(&self.branch),
            user: non_empty(&self.user),
            status: self.status,
        };
        let page = if more { self.page + 1 } else { 1 };
        self.loading = true;
        spawn_load(
            self,
            |s| &mut s.list_load,
            move || target.client()?.pipelines(&target.repo, &filter, page),
            move |this, res, cx| {
                this.loading = false;
                this.refreshing = false;
                match res {
                    Ok(items) if more => {
                        this.more = items.len() >= PAGE_SIZE;
                        this.page = page;
                        let known: HashSet<_> = this.pipelines.iter().map(|p| p.id).collect();
                        this.pipelines
                            .extend(items.into_iter().filter(|p| !known.contains(&p.id)));
                        this.list_error = None;
                    }
                    Ok(items) => {
                        this.list_error = None;
                        if this.pipelines.is_empty() {
                            this.more = items.len() >= PAGE_SIZE;
                        }
                        // Older pages stay, minus what the new page covers (those may have changed status or vanished from a filter).
                        let covered = if items.len() < PAGE_SIZE {
                            u64::MAX
                        } else {
                            items.iter().map(|p| p.id.0).min().unwrap_or(0)
                        };
                        let fresh: HashSet<_> = items.iter().map(|p| p.id).collect();
                        let tail = this
                            .pipelines
                            .drain(..)
                            .filter(|p| p.id.0 < covered && !fresh.contains(&p.id));
                        this.pipelines = items
                            .iter()
                            .cloned()
                            .chain(tail.collect::<Vec<_>>())
                            .collect();
                    }
                    Err(e) => this.list_error = Some(e.into()),
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    /// Start over with another filter.
    pub fn reload_list(&mut self, cx: &mut Context<Self>) {
        self.list_load.cancel();
        self.loading = false;
        self.pipelines.clear();
        self.page = 1;
        self.more = false;
        self.list_error = None;
        self.load_list(false, cx);
    }

    pub fn set_status(&mut self, status: Option<gd_forge::Status>, cx: &mut Context<Self>) {
        self.status = status;
        self.reload_list(cx);
    }

    /// A filter was applied (`ComboPicked`). The forges filter on exact names, so typed text never is (part of a name would match nothing).
    pub fn apply(&mut self, field: Field, value: String, cx: &mut Context<Self>) {
        match field {
            Field::Branch => self.set_filters(value, self.user.clone(), cx),
            Field::User => self.set_filters(self.branch.clone(), value, cx),
        }
        cx.notify();
    }

    /// The filters changed: reload the list once typing pauses.
    pub fn set_filters(&mut self, branch: String, user: String, cx: &mut Context<Self>) {
        if (&branch, &user) == (&self.branch, &self.user) {
            return;
        }
        (self.branch, self.user) = (branch, user);
        if !self.open {
            // The next `show` loads the list again, from page 1.
            self.pipelines.clear();
            self.more = false;
            return;
        }
        self.filter_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(FILTER_DEBOUNCE).await;
            this.update(cx, |s, cx| s.reload_list(cx)).ok();
        }));
    }

    /// The list rows on screen want their stages: load the missing ones, a few at a time.
    pub fn request_details(&mut self, visible: Vec<PipelineId>, cx: &mut Context<Self>) {
        self.queue = visible
            .into_iter()
            .filter(|id| self.needs_detail(*id))
            .collect();
        self.pump_details(cx);
    }

    fn pump_details(&mut self, cx: &mut Context<Self>) {
        while self.inflight.len() < DETAIL_CONCURRENCY && !self.queue.is_empty() {
            let id = self.queue.remove(0);
            self.fetch_detail(id, cx);
        }
    }

    /// Load (or reload) the stages and jobs of a pipeline into the cache.
    pub fn fetch_detail(&mut self, id: PipelineId, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else {
            return;
        };
        if !self.inflight.insert(id) {
            return;
        }
        let epoch = self.epoch;
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { load_detail(&target, id) })
                .await;
            this.update(cx, |this, cx| {
                if this.epoch != epoch {
                    return;
                }
                this.inflight.remove(&id);
                match res {
                    Ok(detail) => {
                        this.failed.remove(&id);
                        this.details.insert(id, detail);
                    }
                    Err(e) => {
                        this.failed.insert(id);
                        this.detail_error = Some(e.into());
                    }
                }
                this.pump_details(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// The pipeline with its stages. Platforms without stages get them from the workflow file in the working tree.
fn load_detail(target: &Target, id: PipelineId) -> gd_forge::Result<PipelineDetail> {
    let client = target.client()?;
    let mut detail = client.pipeline(&target.repo, &id)?;
    if !client.capabilities().stages {
        let yaml = detail.pipeline.workflow_path.as_deref().and_then(|path| {
            workflow_candidates(path)
                .into_iter()
                .find_map(|candidate| std::fs::read_to_string(target.root.join(candidate)).ok())
        });
        if let Some(yaml) = yaml {
            let jobs = detail.stages.drain(..).flat_map(|s| s.jobs).collect();
            detail.stages = stages_from_workflow(&yaml, jobs);
        }
    }
    Ok(detail)
}

/// Indices of the lines to show: everything outside collapsed sections, plus their header lines.
/// A section is collapsed when its `collapsed` flag differs from `toggled` (the sections the user flipped, by index).
pub fn visible_lines(log: &Log, toggled: &HashSet<usize>) -> Vec<usize> {
    let mut visible = Vec::with_capacity(log.lines.len());
    let mut line = 0;
    while line < log.lines.len() {
        visible.push(line);
        let collapsed = log
            .sections
            .iter()
            .enumerate()
            .find(|(ix, s)| s.header == line && s.collapsed != toggled.contains(ix));
        line = collapsed.map_or(line + 1, |(_, s)| s.end.max(line + 1));
    }
    visible
}

/// Paths (relative to the working tree) where the workflow file may be.
fn workflow_candidates(path: &str) -> Vec<String> {
    if path.contains('/') {
        vec![path.to_string()]
    } else {
        WORKFLOW_DIRS
            .iter()
            .map(|dir| format!("{dir}/{path}"))
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/ops.test.rs"]
mod tests;
