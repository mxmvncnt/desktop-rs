//! `impl CiStore`: resolving the account, loading pipelines and details, and the requests behind the buttons.

use std::collections::HashSet;

use gd_forge::{PipelineDetail, PipelineFilter, PipelineId, stages_from_workflow};
use gd_git::Repository;
use gpui::{AppContext, Context};

use super::state::CiStore;
use super::target::Target;
use crate::features::accounts::{Account, RepoAccounts};
use crate::repo::spawn_load;

/// The forges answer 20 pipelines per page.
const PAGE_SIZE: usize = 20;
/// Detail requests for the list's stage dots running at once.
const DETAIL_CONCURRENCY: usize = 4;
/// Where Gitea, Forgejo and GitHub keep workflow files; Forgejo reports a bare file name.
const WORKFLOW_DIRS: [&str; 3] = [".forgejo/workflows", ".gitea/workflows", ".github/workflows"];

impl CiStore {
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.poll = None;
        cx.notify();
    }

    /// Open the page for `repo` (or refresh it): another repository starts from scratch, the same one keeps its cache.
    pub fn show(&mut self, repo: Repository, branch: Option<String>, accounts: Vec<Account>, picks: RepoAccounts, cx: &mut Context<Self>) {
        if self.root.as_deref() != Some(repo.root()) {
            *self = Self {
                epoch: self.epoch + 1,
                focused: self.focused,
                current_branch: self.current_branch,
                poll: self.poll.take(),
                root: Some(repo.root().to_path_buf()),
                ..Self::default()
            };
        }
        self.open = true;
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
                }
                cx.notify();
            },
            cx,
        );
        cx.notify();
    }

    /// Page 1 merged into what is loaded (polling), or the next page (`more`).
    pub fn load_list(&mut self, more: bool, cx: &mut Context<Self>) {
        let Some(target) = self.ready().cloned() else { return };
        if self.loading {
            return;
        }
        let filter = PipelineFilter { ref_name: target.branch.clone().filter(|_| self.current_branch), status: self.status };
        let page = if more { self.page + 1 } else { 1 };
        self.loading = true;
        spawn_load(
            self,
            |s| &mut s.list_load,
            move || target.client()?.pipelines(&target.repo, &filter, page),
            move |this, res, cx| {
                this.loading = false;
                match res {
                    Ok(items) if more => {
                        this.more = items.len() >= PAGE_SIZE;
                        this.page = page;
                        let known: HashSet<_> = this.pipelines.iter().map(|p| p.id).collect();
                        this.pipelines.extend(items.into_iter().filter(|p| !known.contains(&p.id)));
                        this.list_error = None;
                    }
                    Ok(items) => {
                        this.list_error = None;
                        if this.pipelines.is_empty() {
                            this.more = items.len() >= PAGE_SIZE;
                        }
                        // Older pages stay, minus what the new page covers (those may have changed status or vanished from a filter).
                        let covered = if items.len() < PAGE_SIZE { u64::MAX } else { items.iter().map(|p| p.id.0).min().unwrap_or(0) };
                        let fresh: HashSet<_> = items.iter().map(|p| p.id).collect();
                        let tail = this.pipelines.drain(..).filter(|p| p.id.0 < covered && !fresh.contains(&p.id));
                        this.pipelines = items.iter().cloned().chain(tail.collect::<Vec<_>>()).collect();
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

    pub fn toggle_current_branch(&mut self, cx: &mut Context<Self>) {
        self.current_branch = !self.current_branch;
        self.reload_list(cx);
    }

    /// The list rows on screen want their stages: load the missing ones, a few at a time.
    pub fn request_details(&mut self, visible: Vec<PipelineId>, cx: &mut Context<Self>) {
        self.queue = visible.into_iter().filter(|id| self.needs_detail(*id)).collect();
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
        let Some(target) = self.ready().cloned() else { return };
        if !self.inflight.insert(id) {
            return;
        }
        let epoch = self.epoch;
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { load_detail(&target, id) }).await;
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
                    Err(_) => {
                        this.failed.insert(id);
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
            workflow_candidates(path).into_iter().find_map(|candidate| std::fs::read_to_string(target.root.join(candidate)).ok())
        });
        if let Some(yaml) = yaml {
            let jobs = detail.stages.drain(..).flat_map(|s| s.jobs).collect();
            detail.stages = stages_from_workflow(&yaml, jobs);
        }
    }
    Ok(detail)
}

/// Paths (relative to the working tree) where the workflow file may be.
fn workflow_candidates(path: &str) -> Vec<String> {
    if path.contains('/') {
        vec![path.to_string()]
    } else {
        WORKFLOW_DIRS.iter().map(|dir| format!("{dir}/{path}")).collect()
    }
}

#[cfg(test)]
#[path = "tests/ops.test.rs"]
mod tests;
