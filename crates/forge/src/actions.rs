//! GitHub Actions REST API, which Gitea also implements (Gitea >= 1.24 for runs and jobs,
//! 1.25 for rerun). Differences are branched on `Client::forge`.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::client::{Client, Forge, PAGE_SIZE};
use crate::error::Result;
use crate::model::*;
use crate::repo_ref::RepoRef;

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Actor {
    login: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct Run {
    id: u64,
    run_number: u64,
    display_title: Option<String>,
    name: Option<String>,
    path: Option<String>,
    head_branch: Option<String>,
    head_sha: String,
    event: Option<String>,
    status: String,
    conclusion: Option<String>,
    #[serde(alias = "triggering_actor")]
    trigger_actor: Option<Actor>,
    actor: Option<Actor>,
    created_at: Option<DateTime<Utc>>,
    run_started_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    html_url: String,
}

#[derive(Deserialize)]
struct Jobs {
    jobs: Vec<JobJson>,
}

#[derive(Deserialize)]
struct JobJson {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    html_url: Option<String>,
    runner_name: Option<String>,
    #[serde(default)]
    steps: Vec<StepJson>,
}

#[derive(Deserialize)]
struct StepJson {
    name: String,
    status: String,
    conclusion: Option<String>,
    number: u32,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

/// Map a status / conclusion pair. Gitea reports `queued` for waiting tasks and `waiting` for blocked ones.
pub(crate) fn status(status: &str, conclusion: Option<&str>) -> Status {
    match (status, conclusion.unwrap_or("")) {
        ("requested", _) => Status::Created,
        ("queued" | "pending", _) => Status::Pending,
        ("waiting", _) => Status::WaitingForResource,
        ("in_progress", _) => Status::Running,
        (_, "success" | "neutral") => Status::Success,
        (_, "cancelled" | "stale") => Status::Canceled,
        (_, "skipped") => Status::Skipped,
        (_, "action_required") => Status::Manual,
        // failure, timed_out, startup_failure, or a completed run without conclusion
        _ => Status::Failed,
    }
}

pub(crate) fn pipelines(c: &Client, repo: &RepoRef, filter: &PipelineFilter, page: u32) -> Result<Vec<Pipeline>> {
    // GitHub reads `per_page`, Gitea `limit`.
    let mut query = vec![("per_page", PAGE_SIZE.to_string()), ("limit", PAGE_SIZE.to_string()), ("page", page.to_string())];
    if let Some(branch) = &filter.ref_name {
        query.push(("branch", branch.clone()));
    }
    if let Some(s) = filter.status {
        query.push(("status", s.query(false).to_string()));
    }
    let runs: Runs = c.get(&format!("/repos/{}/actions/runs", repo.full_path()), &query)?;
    Ok(runs.workflow_runs.into_iter().map(pipeline_from).collect())
}

pub(crate) fn pipeline(c: &Client, repo: &RepoRef, id: &PipelineId) -> Result<PipelineDetail> {
    let base = format!("/repos/{}/actions/runs/{}", repo.full_path(), id.0);
    let pipeline = pipeline_from(c.get(&base, &[])?);
    let size = PAGE_SIZE.max(100).to_string();
    let jobs: Jobs = c.get(&format!("{base}/jobs"), &[("per_page", size.clone()), ("limit", size), ("filter", "latest".into())])?;
    let jobs: Vec<Job> = jobs.jobs.into_iter().map(|j| job_from(j, *id)).collect();
    let name = pipeline.workflow.clone().unwrap_or_else(|| "Jobs".into());
    Ok(PipelineDetail { stages: vec![Stage { name, status: stage_status(&jobs), jobs }], pipeline })
}

pub(crate) fn job_log(c: &Client, repo: &RepoRef, job: &JobId) -> Result<String> {
    c.text(&format!("/repos/{}/actions/jobs/{}/logs", repo.full_path(), job.id))
}

pub(crate) fn retry_pipeline(c: &Client, repo: &RepoRef, id: &PipelineId) -> Result<()> {
    c.call("POST", &format!("/repos/{}/actions/runs/{}/rerun-failed-jobs", repo.full_path(), id.0), &[]).map(drop)
}

pub(crate) fn cancel_pipeline(c: &Client, repo: &RepoRef, id: &PipelineId) -> Result<()> {
    c.call("POST", &format!("/repos/{}/actions/runs/{}/cancel", repo.full_path(), id.0), &[]).map(drop)
}

pub(crate) fn retry_job(c: &Client, repo: &RepoRef, job: &JobId) -> Result<()> {
    let path = match c.forge {
        Forge::Gitea => format!("/repos/{}/actions/runs/{}/jobs/{}/rerun", repo.full_path(), job.pipeline.0, job.id),
        _ => format!("/repos/{}/actions/jobs/{}/rerun", repo.full_path(), job.id),
    };
    c.call("POST", &path, &[]).map(drop)
}

fn pipeline_from(run: Run) -> Pipeline {
    let status = status(&run.status, run.conclusion.as_deref());
    let started_at = known(run.run_started_at.or(run.started_at));
    let finished_at = known(run.completed_at).or(if status.is_finished() { known(run.updated_at) } else { None });
    // GitHub appends `@ref` to the path of some runs.
    let workflow_path = run.path.map(|p| p.split('@').next().unwrap_or_default().to_string());
    let file = workflow_path.as_deref().and_then(|p| p.rsplit('/').next()).map(str::to_string);
    let user = run.trigger_actor.or(run.actor).map(|a| User { name: a.full_name.filter(|n| !n.is_empty()), username: a.login, avatar_url: a.avatar_url });
    Pipeline {
        id: PipelineId(run.id),
        number: run.run_number,
        title: run.display_title,
        workflow: run.name.or(file),
        workflow_path,
        git_ref: run.head_branch.unwrap_or_default(),
        sha: run.head_sha,
        source: run.event,
        status,
        user,
        created_at: known(run.created_at),
        started_at,
        finished_at,
        duration: seconds(started_at, finished_at),
        web_url: run.html_url,
    }
}

fn job_from(job: JobJson, pipeline: PipelineId) -> Job {
    let started_at = known(job.started_at);
    let finished_at = known(job.completed_at);
    let steps = job
        .steps
        .into_iter()
        .map(|s| Step { name: s.name, status: status(&s.status, s.conclusion.as_deref()), number: s.number, duration: seconds(known(s.started_at), known(s.completed_at)) })
        .collect();
    Job {
        id: JobId { pipeline, id: job.id },
        name: job.name,
        stage: None,
        status: status(&job.status, job.conclusion.as_deref()),
        allow_failure: false,
        needs: Vec::new(),
        started_at,
        finished_at,
        duration: seconds(started_at, finished_at),
        web_url: job.html_url,
        runner: job.runner_name.filter(|r| !r.is_empty()),
        steps,
    }
}

#[cfg(test)]
#[path = "tests/actions.test.rs"]
mod tests;
