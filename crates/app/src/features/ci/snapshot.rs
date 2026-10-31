//! Snapshot views: the CI page filled with fixture data (nothing is fetched).

use chrono::{Duration, Utc};
use gd_forge::{Forge, Job, JobId, Pipeline, PipelineDetail, PipelineId, RepoRef, Stage, Status, User};
use gpui::{Context, Window};

use super::state::{CiStore, CiView, PipelineTab};
use super::target::{Problem, Target};
use crate::features::settings::ThemeMode;
use crate::snapshot::View;
use crate::workspace::Workspace;

pub fn views() -> &'static [View] {
    &[
        View { name: "ci-pipelines*", steps: &[ci_sample] },
        View { name: "ci-pipeline*", steps: &[ci_sample] },
        View { name: "ci-jobs*", steps: &[ci_sample] },
        View { name: "ci-empty-account*", steps: &[ci_sample] },
        View { name: "ci-error*", steps: &[ci_sample] },
    ]
}

fn ci_sample(ws: &mut Workspace, view: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.settings.theme = if view.ends_with("light") { ThemeMode::Light } else { ThemeMode::Dark };
    ws.apply_theme(window, cx);
    ws.ci.update(cx, |ci, cx| {
        ci.open = true;
        if view.starts_with("ci-empty-account") {
            ci.target = Some(Err(Problem::NoAccount("gitlab.example.com".into())));
        } else if view.starts_with("ci-error") {
            ci.target = Some(Err(Problem::Unauthorized));
        } else {
            fill(ci, view);
        }
        cx.notify();
    });
}

fn fill(ci: &mut CiStore, view: &str) {
    ci.target = Some(Ok(Target {
        forge: Forge::GitLab,
        repo: RepoRef { host: "gitlab.example.com".into(), owner: "team".into(), name: "app".into() },
        root: "/tmp/desktop-rs/ci".into(),
        branch: Some("main".into()),
        account_id: "1".into(),
        account_name: "Work".into(),
        host: "gitlab.example.com".into(),
    }));
    let details = sample_pipelines();
    ci.pipelines = details.iter().map(|d| d.pipeline.clone()).collect();
    // The last rows have no stages yet, as when they were just scrolled into view.
    ci.details = details.into_iter().take(7).map(|d| (d.pipeline.id, d)).collect();
    ci.more = true;
    if (view.starts_with("ci-pipeline") && !view.starts_with("ci-pipelines")) || view.starts_with("ci-jobs") {
        let release = release_pipeline();
        ci.view = CiView::Pipeline(release.pipeline.id);
        ci.tab = if view.starts_with("ci-jobs") { PipelineTab::Jobs } else { PipelineTab::Graph };
        ci.details.insert(release.pipeline.id, release);
    }
}

/// Four stages: a failed job that may fail, and a manual deploy.
fn release_pipeline() -> PipelineDetail {
    use Status::*;
    let mut coverage = job(109, 1092, "coverage", Failed, Some(40));
    coverage.allow_failure = true;
    let stages = vec![
        stage("build", vec![job(109, 1090, "compile", Success, Some(61)), job(109, 1091, "compile-docs", Success, Some(48))]),
        stage("test", vec![job(109, 1093, "unit", Success, Some(120)), coverage, job(109, 1094, "lint", Success, Some(24)), job(109, 1095, "integration", Success, Some(260))]),
        stage("package", vec![job(109, 1096, "docker-image", Success, Some(150))]),
        stage("deploy", vec![job(109, 1097, "staging", Success, Some(33)), job(109, 1098, "production", Manual, None)]),
    ];
    detail(109, Success, "Release 1.5.0", "release/1.5", 12, stages)
}

fn job(pipeline: u64, id: u64, name: &str, status: Status, secs: Option<u64>) -> Job {
    Job {
        id: JobId { pipeline: PipelineId(pipeline), id },
        name: name.into(),
        stage: None,
        status,
        allow_failure: false,
        needs: Vec::new(),
        started_at: None,
        finished_at: None,
        duration: secs,
        web_url: None,
        runner: Some("runner-eu-1".into()),
        steps: Vec::new(),
    }
}

fn stage(name: &str, jobs: Vec<Job>) -> Stage {
    let status = jobs.iter().filter(|j| !j.allow_failure).map(|j| j.status).find(|s| *s != Status::Success).unwrap_or(Status::Success);
    Stage { name: name.into(), status, jobs }
}

fn detail(id: u64, status: Status, title: &str, branch: &str, ago_min: i64, stages: Vec<Stage>) -> PipelineDetail {
    let created = Utc::now() - Duration::minutes(ago_min);
    PipelineDetail {
        pipeline: Pipeline {
            id: PipelineId(id),
            number: id,
            title: Some(title.into()),
            workflow: None,
            workflow_path: None,
            git_ref: branch.into(),
            sha: format!("{id:x}a3f9c2d41b7e08"),
            source: Some(if id % 3 == 0 { "merge_request_event" } else { "push" }.into()),
            status,
            user: Some(User { name: Some(["Maxime Vincent", "Ada Lovelace", "Linus T."][id as usize % 3].into()), username: "user".into(), avatar_url: None }),
            created_at: Some(created),
            started_at: Some(created),
            finished_at: None,
            duration: if status == Status::Running { None } else { Some(95 + id * 37) },
            web_url: "https://gitlab.example.com/team/app/-/pipelines/1".into(),
        },
        stages,
    }
}

/// Eight pipelines, newest first, covering the statuses of the list.
fn sample_pipelines() -> Vec<PipelineDetail> {
    use Status::*;
    let build = |p, s| stage("build", vec![job(p, p * 10, "compile", s, Some(61))]);
    let test = |p, s| stage("test", vec![job(p, p * 10 + 1, "unit", s, Some(120)), job(p, p * 10 + 2, "lint", s, Some(24))]);
    let deploy = |p, s| stage("deploy", vec![job(p, p * 10 + 3, "production", s, None)]);
    let mut allowed = job(105, 1050, "coverage", Failed, Some(40));
    allowed.allow_failure = true;
    vec![
        detail(108, Running, "Add the pipelines page", "feat/ci", 2, vec![build(108, Success), test(108, Running), deploy(108, Created)]),
        detail(107, Failed, "Fix the log parser on CRLF", "main", 25, vec![build(107, Success), test(107, Failed), deploy(107, Skipped)]),
        detail(106, Success, "Bump dependencies", "main", 70, vec![build(106, Success), test(106, Success), deploy(106, Success)]),
        detail(105, Success, "Refactor the cache", "feat/cache", 190, vec![build(105, Success), stage("test", vec![allowed, job(105, 1051, "unit", Success, Some(90))])]),
        detail(104, Manual, "Release 1.4.0", "release/1.4", 600, vec![build(104, Success), test(104, Success), deploy(104, Manual)]),
        detail(103, Canceled, "WIP: new sidebar", "feat/sidebar", 1500, vec![build(103, Success), test(103, Canceled)]),
        detail(102, Pending, "Update README", "docs", 3000, vec![build(102, Pending)]),
        detail(101, Success, "Initial commit", "main", 9000, vec![build(101, Success)]),
        detail(100, Success, "Prepare the release", "main", 20000, vec![build(100, Success)]),
    ]
}
