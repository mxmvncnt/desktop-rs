use super::*;

#[test]
fn pipeline_detail() {
    let p: Pl = serde_json::from_str(include_str!("fixtures/gitlab_pipeline.json")).unwrap();
    let p = pipeline_from(p, Some("feat: ship it".into()));
    assert_eq!((p.number, p.status, p.duration), (3, Status::Success, Some(175)));
    assert_eq!(p.title.as_deref(), Some("feat: ship it"));
    assert_eq!(p.user.unwrap().username, "root");
    assert_eq!(p.source.as_deref(), Some("push"));
}

#[test]
fn statuses_parse() {
    for (s, expected) in [("waiting_for_resource", Status::WaitingForResource), ("canceling", Status::Canceled), ("canceled", Status::Canceled), ("manual", Status::Manual)] {
        assert_eq!(serde_json::from_str::<Status>(&format!("\"{s}\"")).unwrap(), expected);
    }
}

#[test]
fn job_fields() {
    let jobs: Vec<JobJson> = serde_json::from_str(include_str!("fixtures/gitlab_jobs.json")).unwrap();
    let rspec = job_from(jobs.into_iter().nth(1).unwrap(), PipelineId(47));
    assert_eq!((rspec.stage.as_deref(), rspec.status, rspec.allow_failure, rspec.duration), (Some("test"), Status::Failed, true, Some(60)));
    assert_eq!(rspec.runner.as_deref(), Some("shared-runner-1"));
}

#[test]
fn passed_with_warnings() {
    let pipeline = pipeline_from(serde_json::from_str(include_str!("fixtures/gitlab_pipeline.json")).unwrap(), None);
    let jobs: Vec<JobJson> = serde_json::from_str(include_str!("fixtures/gitlab_jobs.json")).unwrap();
    let jobs: Vec<Job> = jobs.into_iter().map(|j| job_from(j, PipelineId(47))).collect();
    let stages = |jobs: Vec<Job>| vec![Stage { name: "test".into(), status: stage_status(&jobs), jobs }];
    let detail = PipelineDetail { pipeline: pipeline.clone(), stages: stages(vec![jobs[1].clone()]) };
    assert!(detail.passed_with_warnings());
    let detail = PipelineDetail { pipeline, stages: stages(vec![jobs[2].clone()]) };
    assert!(!detail.passed_with_warnings());
}
