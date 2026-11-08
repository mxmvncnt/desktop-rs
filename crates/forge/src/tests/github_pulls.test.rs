use super::*;

#[test]
fn merged_pull_from_a_fork() {
    let p = pull_from(serde_json::from_str(include_str!("fixtures/github_pull.json")).unwrap());
    assert_eq!(
        (p.number, p.state, p.draft),
        (1347, PullState::Merged, false)
    );
    assert!(p.from_fork);
    assert_eq!(p.head_ref.as_deref(), Some("new-topic"));
    assert_eq!(p.base_ref.as_deref(), Some("main"));
    assert_eq!(p.labels, ["bug", "ui"]);
    assert_eq!(p.comments, Some(10));
    assert_eq!(p.author.unwrap().username, "octocat");
}

#[test]
fn replies_join_their_thread() {
    let comments: Vec<ReviewComment> =
        serde_json::from_str(include_str!("fixtures/github_review_comments.json")).unwrap();
    let threads = threads(comments);
    assert_eq!(threads.len(), 2);
    let t = &threads[0];
    assert_eq!((t.id.as_str(), t.path.as_str()), ("10", "src/main.rs"));
    assert_eq!(
        (t.side, t.line, t.start_line),
        (Side::Right, Some(12), Some(10))
    );
    assert_eq!(t.comments.len(), 2);
    assert_eq!(t.comments[1].body, "It reads better.");
    // Outdated: no current line, the original one is kept.
    let t = &threads[1];
    assert_eq!(
        (t.side, t.line, t.original_line),
        (Side::Left, None, Some(3))
    );
}

#[test]
fn search_results_have_no_branches() {
    let item: SearchItem = serde_json::from_str(
        r#"{"number": 5, "title": "t", "body": null, "state": "closed", "draft": false,
            "user": {"login": "ada"}, "labels": [], "assignees": [{"login": "grace"}], "comments": 2,
            "pull_request": {"merged_at": "2025-04-10T12:00:00Z"},
            "created_at": null, "updated_at": null, "html_url": "https://github.com/o/r/pull/5"}"#,
    )
    .unwrap();
    let p = search_item(item);
    assert_eq!((p.number, p.state), (5, PullState::Merged));
    assert_eq!(p.assignees, ["grace"]);
    assert_eq!(p.head_ref, None);
}

#[test]
fn open_pull_merge_status() {
    let mut json: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/github_pull.json")).unwrap();
    json["merged_at"] = serde_json::Value::Null;
    json["state"] = "open".into();
    json["milestone"] = serde_json::json!({"title": "v2"});
    for (state, status) in [
        ("clean", MergeStatus::Mergeable),
        ("dirty", MergeStatus::Conflicts),
        ("blocked", MergeStatus::Blocked),
        ("unstable", MergeStatus::ChecksPending),
        ("unknown", MergeStatus::Checking),
    ] {
        json["mergeable_state"] = state.into();
        let p = pull_from(serde_json::from_value(json.clone()).unwrap());
        assert_eq!(p.merge, Some(status), "{state}");
        assert_eq!(p.milestone.as_deref(), Some("v2"));
    }
}
