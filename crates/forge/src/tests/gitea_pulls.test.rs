use super::*;

#[test]
fn open_pull_from_a_fork() {
    let p = pull_from(serde_json::from_str(include_str!("fixtures/gitea_pull.json")).unwrap());
    assert_eq!((p.number, p.state), (3, PullState::Open));
    assert!(p.from_fork);
    assert_eq!(p.head_sha.as_deref(), Some("1111"));
    // An empty full name is no name.
    assert_eq!(p.author.unwrap().name, None);
}

#[test]
fn comments_on_one_line_are_a_thread() {
    let comments: Vec<ReviewComment> =
        serde_json::from_str(include_str!("fixtures/gitea_review_comments.json")).unwrap();
    let threads = threads(comments);
    assert_eq!(threads.len(), 2);
    assert_eq!((threads[0].side, threads[0].line), (Side::Right, Some(4)));
    assert_eq!(threads[0].comments.len(), 2);
    assert!(threads[0].resolved);
    assert_eq!((threads[1].side, threads[1].line), (Side::Left, Some(2)));
}
