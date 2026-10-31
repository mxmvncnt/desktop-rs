use super::*;
use crate::workflow::group_by_needs;

#[test]
fn status_mapping() {
    for (s, expected) in [("success", Status::Success), ("failure", Status::Failed), ("cancelled", Status::Canceled), ("skipped", Status::Skipped), ("running", Status::Running), ("waiting", Status::Pending), ("blocked", Status::Created), ("unknown", Status::Created)] {
        assert_eq!(status(s), expected, "{s}");
    }
}

#[test]
fn run() {
    let runs: Runs = serde_json::from_str(include_str!("fixtures/forgejo_runs.json")).unwrap();
    let p = pipeline_from(runs.workflow_runs.into_iter().next().unwrap());
    assert_eq!((p.number, p.status, p.duration), (9, Status::Failed, Some(120)));
    assert_eq!(p.git_ref, "main");
    assert_eq!(p.user.unwrap().name.as_deref(), Some("Carol C"));
}

#[test]
fn jobs_are_staged_by_needs() {
    let jobs: Vec<JobJson> = serde_json::from_str(include_str!("fixtures/forgejo_jobs.json")).unwrap();
    assert_eq!(jobs[2].needs, ["build"]);
    let jobs = jobs
        .into_iter()
        .map(|j| Job {
            id: JobId { pipeline: PipelineId(501), id: j.id },
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
    let stages = group_by_needs(jobs);
    assert_eq!(stages.iter().map(|s| s.jobs[0].name.as_str()).collect::<Vec<_>>(), ["lint", "build", "deploy"]);
    assert_eq!(stages[1].status, Status::Failed);
}
