use super::*;

fn job(id: u64, name: &str, status: Status) -> Job {
    Job {
        id: JobId {
            pipeline: PipelineId(1),
            id,
        },
        name: name.into(),
        stage: None,
        status,
        allow_failure: false,
        needs: Vec::new(),
        started_at: None,
        finished_at: None,
        duration: None,
        web_url: None,
        runner: None,
        steps: Vec::new(),
    }
}

const YAML: &str = "
name: CI
jobs:
  lint:
    runs-on: ubuntu-latest
  test:
    name: Test
    needs: lint
    runs-on: ubuntu-latest
  docs:
    needs: [lint]
    continue-on-error: true
  deploy:
    needs: [test, docs]
";

#[test]
fn columns_by_needs_depth() {
    let jobs = vec![
        job(1, "deploy", Status::Created),
        job(2, "Test (ubuntu, 1.80)", Status::Success),
        job(3, "lint", Status::Success),
        job(4, "Test (windows, 1.80)", Status::Running),
        job(5, "docs", Status::Failed),
    ];
    let stages = stages_from_workflow(YAML, jobs);
    let names: Vec<Vec<&str>> = stages
        .iter()
        .map(|s| s.jobs.iter().map(|j| j.name.as_str()).collect())
        .collect();
    assert_eq!(
        names,
        [
            vec!["lint"],
            vec!["Test (ubuntu, 1.80)", "Test (windows, 1.80)", "docs"],
            vec!["deploy"]
        ]
    );
    assert_eq!(stages[1].name, "Stage 2");
    assert_eq!(stages[1].jobs[0].stage.as_deref(), Some("Stage 2"));
    // docs is `continue-on-error`, so its failure doesn't fail the stage
    assert_eq!(stages[1].status, Status::Running);
    assert!(stages[1].jobs[2].allow_failure);
}

#[test]
fn unparsable_file_gives_one_stage() {
    let stages = stages_from_workflow(
        "not: [valid",
        vec![job(1, "a", Status::Success), job(2, "b", Status::Failed)],
    );
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0].jobs.len(), 2);
    assert_eq!(stages[0].status, Status::Failed);
}

#[test]
fn unknown_jobs_and_cycles_do_not_hang() {
    let yaml = "jobs:\n  a:\n    needs: b\n  b:\n    needs: a\n";
    let stages = stages_from_workflow(
        yaml,
        vec![
            job(1, "a", Status::Success),
            job(2, "b", Status::Success),
            job(3, "other", Status::Success),
        ],
    );
    assert_eq!(stages.iter().map(|s| s.jobs.len()).sum::<usize>(), 3);
    assert_eq!(stages[0].jobs[0].name, "other");
}

#[test]
fn stage_status_rules() {
    let mut warned = job(1, "a", Status::Failed);
    warned.allow_failure = true;
    assert_eq!(
        stage_status(&[warned.clone(), job(2, "b", Status::Success)]),
        Status::Success
    );
    assert_eq!(
        stage_status(&[warned, job(2, "b", Status::Pending)]),
        Status::Pending
    );
    assert_eq!(
        stage_status(&[job(1, "a", Status::Skipped)]),
        Status::Skipped
    );
    assert_eq!(
        stage_status(&[job(1, "a", Status::Manual), job(2, "b", Status::Success)]),
        Status::Manual
    );
}
