//! Forgejo's Actions API (Forgejo >= 12 for runs and jobs): runs, jobs with `needs`, logs and run cancel.
//! Logs and cancel share the GitHub-style paths, see `actions`.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::client::{Client, PAGE_SIZE};
use crate::error::Result;
use crate::model::*;
use crate::repo_ref::RepoRef;
use crate::workflow::group_by_needs;

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    id: u64,
    index_in_repo: u64,
    title: Option<String>,
    prettyref: Option<String>,
    commit_sha: String,
    event: Option<String>,
    workflow_id: Option<String>,
    status: String,
    trigger_user: Option<TriggerUser>,
    created: Option<DateTime<Utc>>,
    started: Option<DateTime<Utc>>,
    stopped: Option<DateTime<Utc>>,
    /// Nanoseconds.
    duration: Option<u64>,
    html_url: String,
}

#[derive(Deserialize)]
struct TriggerUser {
    login: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct JobJson {
    id: u64,
    name: String,
    status: String,
    #[serde(default)]
    needs: Vec<String>,
}

fn status(status: &str) -> Status {
    match status {
        "success" => Status::Success,
        "failure" => Status::Failed,
        "cancelled" => Status::Canceled,
        "skipped" => Status::Skipped,
        "running" => Status::Running,
        "waiting" => Status::Pending,
        "blocked" => Status::Created,
        // "unknown" and anything new
        _ => Status::Created,
    }
}

pub(crate) fn pipelines(c: &Client, repo: &RepoRef, filter: &PipelineFilter, page: u32) -> Result<Vec<Pipeline>> {
    let mut query = vec![("limit", PAGE_SIZE.to_string()), ("page", page.to_string())];
    if let Some(name) = &filter.ref_name {
        query.push(("ref", if name.starts_with("refs/") { name.clone() } else { format!("refs/heads/{name}") }));
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
    let jobs: Vec<JobJson> = c.get(&format!("{base}/jobs"), &[])?;
    let jobs = jobs
        .into_iter()
        .map(|j| Job {
            id: JobId { pipeline: *id, id: j.id },
            name: j.name,
            stage: None,
            status: status(&j.status),
            allow_failure: false,
            needs: j.needs,
            started_at: None,
            finished_at: None,
            duration: None,
            web_url: None,
            runner: None,
            steps: Vec::new(),
        })
        .collect();
    Ok(PipelineDetail { stages: group_by_needs(jobs), pipeline })
}

fn pipeline_from(run: Run) -> Pipeline {
    let user = run.trigger_user.map(|u| User { name: u.full_name.filter(|n| !n.is_empty()), username: u.login, avatar_url: u.avatar_url });
    Pipeline {
        id: PipelineId(run.id),
        number: run.index_in_repo,
        title: run.title,
        workflow: run.workflow_id.clone(),
        workflow_path: run.workflow_id,
        git_ref: run.prettyref.unwrap_or_default(),
        sha: run.commit_sha,
        source: run.event,
        status: status(&run.status),
        user,
        created_at: known(run.created),
        started_at: known(run.started),
        finished_at: known(run.stopped),
        duration: run.duration.map(|ns| ns / 1_000_000_000),
        web_url: run.html_url,
    }
}

#[cfg(test)]
#[path = "tests/forgejo.test.rs"]
mod tests;
