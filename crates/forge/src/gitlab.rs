//! GitLab REST API v4 (GitLab >= 15 for `iid`, `source` and bridges).

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::client::{Client, PAGE_SIZE};
use crate::error::Result;
use crate::model::*;
use crate::repo_ref::RepoRef;

#[derive(Deserialize)]
struct Pl {
    id: u64,
    iid: u64,
    name: Option<String>,
    #[serde(rename = "ref")]
    git_ref: Option<String>,
    sha: String,
    source: Option<String>,
    status: Status,
    // detail only
    user: Option<UserJson>,
    created_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    duration: Option<f64>,
    web_url: String,
}

#[derive(Deserialize)]
struct UserJson {
    name: Option<String>,
    username: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct JobJson {
    id: u64,
    name: String,
    stage: String,
    status: Status,
    #[serde(default)]
    allow_failure: bool,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    duration: Option<f64>,
    web_url: Option<String>,
    runner: Option<Runner>,
}

#[derive(Deserialize)]
struct Runner {
    description: Option<String>,
}

#[derive(Deserialize)]
struct Commit {
    title: String,
}

pub(crate) fn pipelines(
    c: &Client,
    repo: &RepoRef,
    filter: &PipelineFilter,
    page: u32,
) -> Result<Vec<Pipeline>> {
    let mut query = vec![
        ("per_page", PAGE_SIZE.to_string()),
        ("page", page.to_string()),
        ("order_by", "id".into()),
        ("sort", "desc".into()),
    ];
    if let Some(name) = &filter.ref_name {
        query.push(("ref", name.clone()));
    }
    if let Some(user) = &filter.user {
        query.push(("username", user.clone()));
    }
    if let Some(s) = filter.status {
        query.push(("status", s.query(true).to_string()));
    }
    let list: Vec<Pl> = c.get(&format!("/projects/{}/pipelines", repo.gitlab_id()), &query)?;
    Ok(list.into_iter().map(|p| pipeline_from(p, None)).collect())
}

pub(crate) fn pipeline(c: &Client, repo: &RepoRef, id: &PipelineId) -> Result<PipelineDetail> {
    let project = format!("/projects/{}", repo.gitlab_id());
    let detail: Pl = c.get(&format!("{project}/pipelines/{}", id.0), &[])?;
    let title = c
        .get::<Commit>(&format!("{project}/repository/commits/{}", detail.sha), &[])
        .ok()
        .map(|c| c.title);
    // Retried jobs are included only to learn the stage order: a stage whose single job was retried would otherwise sort last.
    let query = [
        ("per_page", "100".to_string()),
        ("include_retried", "true".into()),
    ];
    let mut jobs: Vec<JobJson> = c.get(&format!("{project}/pipelines/{}/jobs", id.0), &query)?;
    jobs.extend(c.get::<Vec<JobJson>>(
        &format!("{project}/pipelines/{}/bridges", id.0),
        &[("per_page", "100".to_string())],
    )?);
    // Jobs are created in stage order, so ascending ids give the stage order.
    jobs.sort_by_key(|j| j.id);
    let mut stages: Vec<Stage> = Vec::new();
    for job in jobs {
        let i = stages
            .iter()
            .position(|s| s.name == job.stage)
            .unwrap_or_else(|| {
                stages.push(Stage {
                    name: job.stage.clone(),
                    status: Status::Created,
                    jobs: Vec::new(),
                });
                stages.len() - 1
            });
        let job = job_from(job, *id);
        match stages[i].jobs.iter_mut().find(|j| j.name == job.name) {
            Some(older) => *older = job,
            None => stages[i].jobs.push(job),
        }
    }
    for stage in &mut stages {
        stage.status = stage_status(&stage.jobs);
    }
    Ok(PipelineDetail {
        pipeline: pipeline_from(detail, title),
        stages,
    })
}

pub(crate) fn job_log(c: &Client, repo: &RepoRef, job: &JobId) -> Result<String> {
    c.text(&format!(
        "/projects/{}/jobs/{}/trace",
        repo.gitlab_id(),
        job.id
    ))
}

/// POST `path` below the project.
pub(crate) fn post(c: &Client, repo: &RepoRef, path: &str) -> Result<()> {
    c.call(
        "POST",
        &format!("/projects/{}{path}", repo.gitlab_id()),
        &[],
    )
    .map(drop)
}

fn pipeline_from(p: Pl, commit_title: Option<String>) -> Pipeline {
    Pipeline {
        id: PipelineId(p.id),
        number: p.iid,
        title: commit_title.or(p.name),
        workflow: None,
        workflow_path: None,
        git_ref: p.git_ref.unwrap_or_default(),
        sha: p.sha,
        source: p.source,
        status: p.status,
        user: p.user.map(|u| User {
            name: u.name,
            username: u.username,
            avatar_url: u.avatar_url,
        }),
        created_at: p.created_at,
        started_at: p.started_at,
        finished_at: p.finished_at,
        duration: p
            .duration
            .map(|d| d as u64)
            .or(seconds(p.started_at, p.finished_at)),
        web_url: p.web_url,
    }
}

fn job_from(j: JobJson, pipeline: PipelineId) -> Job {
    Job {
        id: JobId { pipeline, id: j.id },
        name: j.name,
        stage: Some(j.stage),
        status: j.status,
        allow_failure: j.allow_failure,
        needs: Vec::new(),
        started_at: j.started_at,
        finished_at: j.finished_at,
        duration: j.duration.map(|d| d as u64),
        web_url: j.web_url,
        runner: j
            .runner
            .and_then(|r| r.description)
            .filter(|d| !d.is_empty()),
        steps: Vec::new(),
    }
}

#[cfg(test)]
#[path = "tests/gitlab.test.rs"]
mod tests;
