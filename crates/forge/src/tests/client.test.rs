use super::*;

#[test]
fn api_bases() {
    for (forge, host, base) in [
        (Forge::GitHub, "github.com", "https://api.github.com"),
        (
            Forge::GitHub,
            "ghe.example.com",
            "https://ghe.example.com/api/v3",
        ),
        (Forge::GitLab, "gitlab.com", "https://gitlab.com/api/v4"),
        (Forge::Gitea, "gitea.com", "https://gitea.com/api/v1"),
        (
            Forge::Forgejo,
            "codeberg.org",
            "https://codeberg.org/api/v1",
        ),
    ] {
        assert_eq!(Client::new(forge, host, "t").base, base);
    }
}

#[test]
fn unsupported_actions() {
    let repo = RepoRef {
        host: "codeberg.org".into(),
        owner: "o".into(),
        name: "r".into(),
    };
    let job = JobId {
        pipeline: PipelineId(1),
        id: 2,
    };
    let c = Client::new(Forge::Forgejo, "codeberg.org", "t");
    assert!(matches!(
        c.retry_job(&repo, &job),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        c.play_job(&repo, &job),
        Err(Error::Unsupported(_))
    ));
    assert!(!c.capabilities().retry_pipeline);
}

#[test]
fn status_queries() {
    assert_eq!(Status::Failed.query(true), "failed");
    assert_eq!(Status::Failed.query(false), "failure");
    assert_eq!(Status::Running.query(false), "in_progress");
    assert_eq!(
        Status::WaitingForResource.query(true),
        "waiting_for_resource"
    );
}
