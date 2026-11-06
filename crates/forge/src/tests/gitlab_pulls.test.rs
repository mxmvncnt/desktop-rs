use super::*;

#[test]
fn draft_merge_request_with_diff_refs() {
    let p = pull_from(serde_json::from_str(include_str!("fixtures/gitlab_mr.json")).unwrap());
    assert_eq!((p.number, p.state, p.draft), (7, PullState::Open, true));
    assert!(!p.from_fork);
    assert_eq!(p.head_ref.as_deref(), Some("fast-parser"));
    assert_eq!(p.author.unwrap().name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(p.diff_refs.unwrap().start_sha, "cccc");
}

#[test]
fn discussions_split_into_comments_and_threads() {
    let d: Vec<Discussion> =
        serde_json::from_str(include_str!("fixtures/gitlab_discussions.json")).unwrap();
    let c = conversation_from(d);
    // The system note is left out.
    assert_eq!(c.comments.len(), 1);
    assert_eq!(c.comments[0].body, "Looks good overall");
    assert_eq!(c.threads.len(), 1);
    let t = &c.threads[0];
    assert_eq!((t.id.as_str(), t.path.as_str()), ("d3", "src/lib.rs"));
    assert_eq!(
        (t.side, t.line, t.start_line),
        (Side::Right, Some(40), Some(38))
    );
    assert!(t.resolved);
    assert_eq!(t.comments.len(), 2);
}

#[test]
fn range_position_names_its_ends_by_line_code() {
    let refs = DiffRefs {
        base_sha: "b".into(),
        start_sha: "s".into(),
        head_sha: "h".into(),
    };
    let target = LineTarget {
        path: "a.rs".into(),
        old_path: "a.rs".into(),
        start: LinePos {
            old: Some(3),
            new: Some(3),
        },
        end: LinePos {
            old: None,
            new: Some(5),
        },
    };
    let pos = position(&refs, &target);
    assert_eq!(pos["new_line"], 5);
    assert!(pos.get("old_line").is_none());
    let file = sha1_smol::Sha1::from("a.rs").hexdigest();
    assert_eq!(
        pos["line_range"]["start"]["line_code"],
        format!("{file}_3_3")
    );
    assert_eq!(pos["line_range"]["start"]["type"], serde_json::Value::Null);
    assert_eq!(pos["line_range"]["end"]["line_code"], format!("{file}__5"));
    assert_eq!(pos["line_range"]["end"]["type"], "new");
    // One line: no range.
    let one = LineTarget {
        start: target.end,
        ..target
    };
    assert!(position(&refs, &one).get("line_range").is_none());
}
