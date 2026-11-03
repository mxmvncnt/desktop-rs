use super::*;

#[test]
fn status_mapping() {
    let table = [
        ("queued", None, Status::Pending),
        ("requested", None, Status::Created),
        ("waiting", None, Status::WaitingForResource),
        ("in_progress", None, Status::Running),
        ("completed", Some("success"), Status::Success),
        ("completed", Some("neutral"), Status::Success),
        ("completed", Some("failure"), Status::Failed),
        ("completed", Some("timed_out"), Status::Failed),
        ("completed", Some("startup_failure"), Status::Failed),
        ("completed", Some("cancelled"), Status::Canceled),
        ("completed", Some("skipped"), Status::Skipped),
        ("completed", Some("action_required"), Status::Manual),
        ("completed", None, Status::Failed),
    ];
    for (s, c, expected) in table {
        assert_eq!(status(s, c), expected, "{s} {c:?}");
    }
}

#[test]
fn github_run() {
    let p = pipeline_from(serde_json::from_str(include_str!("fixtures/github_run.json")).unwrap());
    assert_eq!(p.id, PipelineId(9876543210));
    assert_eq!((p.number, p.status), (42, Status::Failed));
    assert_eq!(p.title.as_deref(), Some("feat: add the CI page"));
    assert_eq!(p.workflow.as_deref(), Some("CI"));
    assert_eq!(p.workflow_path.as_deref(), Some(".github/workflows/ci.yml"));
    assert_eq!(p.user.unwrap().username, "hubot");
    assert_eq!(p.duration, Some(325));
    assert_eq!(p.git_ref, "feat/ci");
}

#[test]
fn github_jobs() {
    let jobs: Jobs = serde_json::from_str(include_str!("fixtures/github_jobs.json")).unwrap();
    let jobs: Vec<Job> = jobs
        .jobs
        .into_iter()
        .map(|j| job_from(j, PipelineId(1)))
        .collect();
    assert_eq!(
        jobs.iter().map(|j| j.status).collect::<Vec<_>>(),
        [Status::Success, Status::Failed, Status::Skipped]
    );
    assert_eq!(jobs[0].duration, Some(40));
    assert_eq!(jobs[0].steps.len(), 2);
    assert_eq!(jobs[0].steps[1].duration, Some(36));
    assert_eq!(jobs[2].runner, None);
    assert_eq!(
        jobs[1].id,
        JobId {
            pipeline: PipelineId(1),
            id: 112
        }
    );
}

#[test]
fn gitea_run() {
    let p = pipeline_from(serde_json::from_str(include_str!("fixtures/gitea_run.json")).unwrap());
    assert_eq!(p.status, Status::Success);
    assert_eq!(p.workflow.as_deref(), Some("build.yaml"));
    assert_eq!(
        p.workflow_path.as_deref(),
        Some(".gitea/workflows/build.yaml")
    );
    assert_eq!(p.duration, Some(60));
    let user = p.user.unwrap();
    assert_eq!(
        (user.username.as_str(), user.name.as_deref()),
        ("alice", Some("Alice A"))
    );
}
