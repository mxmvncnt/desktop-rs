use gd_forge::Forge;

use super::*;

fn job(status: Status) -> Job {
    Job {
        id: JobId {
            pipeline: PipelineId(1),
            id: 2,
        },
        name: "build".into(),
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

fn labels(status: Status, forge: Forge) -> Vec<&'static str> {
    job_actions(&job(status), forge.capabilities())
        .into_iter()
        .map(|(_, _, label)| label)
        .collect()
}

#[test]
fn gitlab_plays_manual_jobs_and_retries_finished_ones() {
    assert_eq!(labels(Status::Manual, Forge::GitLab), ["Play"]);
    assert_eq!(labels(Status::Failed, Forge::GitLab), ["Retry"]);
    assert_eq!(labels(Status::Running, Forge::GitLab), ["Cancel"]);
}

#[test]
fn platforms_only_offer_what_their_api_can_do() {
    assert_eq!(labels(Status::Failed, Forge::GitHub), ["Retry"]);
    assert!(labels(Status::Running, Forge::GitHub).is_empty());
    assert!(labels(Status::Manual, Forge::Gitea).is_empty());
    assert!(labels(Status::Failed, Forge::Forgejo).is_empty());
}
