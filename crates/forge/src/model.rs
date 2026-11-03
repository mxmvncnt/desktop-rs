//! The normalized CI model, in GitLab's vocabulary.

use chrono::{DateTime, Datelike, Utc};
use serde::Deserialize;

/// Pipeline / job status. Serde names are GitLab's, which is what GitLab returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Created,
    Pending,
    WaitingForResource,
    Preparing,
    Manual,
    Scheduled,
    Running,
    Success,
    Failed,
    #[serde(alias = "canceling")]
    Canceled,
    Skipped,
}

impl Status {
    /// Not going to change any more.
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            Status::Success | Status::Failed | Status::Canceled | Status::Skipped
        )
    }

    /// The value of the `status` query parameter that selects this status. GitLab uses its own names;
    /// GitHub, Gitea and Forgejo accept a status or a conclusion.
    pub(crate) fn query(self, gitlab: bool) -> &'static str {
        match (self, gitlab) {
            (Status::Created, true) => "created",
            (Status::Pending, true) => "pending",
            (Status::WaitingForResource, true) => "waiting_for_resource",
            (Status::Preparing, true) => "preparing",
            (Status::Manual, true) => "manual",
            (Status::Scheduled, true) => "scheduled",
            (Status::Running, true) => "running",
            (Status::Success, _) => "success",
            (Status::Failed, true) => "failed",
            (Status::Failed, false) => "failure",
            (Status::Canceled, true) => "canceled",
            (Status::Canceled, false) => "cancelled",
            (Status::Skipped, _) => "skipped",
            (Status::Created, false) => "requested",
            (Status::Pending, false) => "queued",
            (Status::WaitingForResource, false) => "waiting",
            (Status::Preparing, false) => "pending",
            (Status::Manual, false) => "action_required",
            (Status::Scheduled, false) => "queued",
            (Status::Running, false) => "in_progress",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(pub u64);

/// A job id. Gitea needs the run in its job-rerun URL, so the pipeline travels with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId {
    pub pipeline: PipelineId,
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub name: Option<String>,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub id: PipelineId,
    /// GitLab `iid`, GitHub/Gitea `run_number`, Forgejo `index_in_repo`.
    pub number: u64,
    /// Commit title (GitHub/Gitea/Forgejo) or pipeline name (GitLab); `pipeline()` fills the commit title.
    pub title: Option<String>,
    /// Workflow name (GitHub/Gitea/Forgejo).
    pub workflow: Option<String>,
    /// Workflow file as the API reports it (`.github/workflows/ci.yml`; Forgejo: the bare file name).
    pub workflow_path: Option<String>,
    pub git_ref: String,
    pub sha: String,
    /// GitLab `source`, or the event that triggered the run (`push`, `pull_request`, ...).
    pub source: Option<String>,
    pub status: Status,
    pub user: Option<User>,
    pub created_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Seconds. For a running pipeline, compute it from `started_at`.
    pub duration: Option<u64>,
    pub web_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub name: String,
    pub status: Status,
    pub number: u32,
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    /// GitLab stage name; `None` until `stages_from_workflow` / the needs grouping assigns one.
    pub stage: Option<String>,
    pub status: Status,
    pub allow_failure: bool,
    /// Names of the jobs this one waits for (Forgejo; filled from the workflow file by `stages_from_workflow`).
    pub needs: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration: Option<u64>,
    pub web_url: Option<String>,
    pub runner: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    pub name: String,
    pub status: Status,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineDetail {
    pub pipeline: Pipeline,
    /// In execution order.
    pub stages: Vec<Stage>,
}

impl PipelineDetail {
    /// "Passed with warnings": the pipeline succeeded but a job that is allowed to fail did not.
    pub fn passed_with_warnings(&self) -> bool {
        self.pipeline.status == Status::Success
            && self
                .stages
                .iter()
                .flat_map(|s| &s.jobs)
                .any(|j| j.allow_failure && j.status == Status::Failed)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PipelineFilter {
    pub ref_name: Option<String>,
    /// Login of who triggered the pipeline; only where `Capabilities::filter_user`.
    pub user: Option<String>,
    pub status: Option<Status>,
}

/// What the platform's API can do. Static per platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Retry the failed jobs of a pipeline.
    pub retry_pipeline: bool,
    pub retry_job: bool,
    pub cancel_pipeline: bool,
    pub cancel_job: bool,
    /// Start a manual job.
    pub play_job: bool,
    pub job_logs: bool,
    /// Jobs carry their steps.
    pub steps: bool,
    /// `pipeline()` returns real stages (otherwise one stage; see `stages_from_workflow`).
    pub stages: bool,
    /// The list can be filtered by who triggered the pipeline.
    pub filter_user: bool,
}

/// Status of a stage from its jobs, like GitLab: failed jobs that are allowed to fail don't fail the stage.
pub(crate) fn stage_status(jobs: &[Job]) -> Status {
    let all: Vec<Status> = jobs
        .iter()
        .filter(|j| !(j.allow_failure && j.status == Status::Failed))
        .map(|j| j.status)
        .collect();
    let any = |s| all.contains(&s);
    for s in [
        Status::Failed,
        Status::Running,
        Status::Preparing,
        Status::WaitingForResource,
        Status::Pending,
        Status::Manual,
        Status::Scheduled,
        Status::Created,
    ] {
        if any(s) {
            return s;
        }
    }
    if any(Status::Canceled) {
        Status::Canceled
    } else if any(Status::Success) || all.is_empty() && !jobs.is_empty() {
        Status::Success
    } else {
        Status::Skipped
    }
}

/// Seconds between two timestamps.
pub(crate) fn seconds(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Option<u64> {
    u64::try_from((to? - from?).num_seconds()).ok()
}

/// Gitea writes unset times as year 1.
pub(crate) fn known(t: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    t.filter(|t| t.year() > 1970)
}
