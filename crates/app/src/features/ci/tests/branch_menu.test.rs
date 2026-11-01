use super::*;

fn branch(name: &str, kind: BranchKind) -> Branch {
    Branch { name: name.into(), kind, is_head: false, upstream: None, tip_time: 0 }
}

#[test]
fn remote_branches_that_are_local_are_listed_once_and_head_is_skipped() {
    let branches = [
        branch("main", BranchKind::Local),
        branch("feat/ci", BranchKind::Local),
        branch("origin/HEAD", BranchKind::Remote),
        branch("origin/main", BranchKind::Remote),
        branch("origin/release/1.5", BranchKind::Remote),
        branch("upstream/main", BranchKind::Remote),
    ];
    assert_eq!(branch_names(&branches, None), ["main", "feat/ci", "release/1.5"]);
}

#[test]
fn the_current_branch_comes_first() {
    let branches = [branch("a", BranchKind::Local), branch("b", BranchKind::Local), branch("c", BranchKind::Remote)];
    assert_eq!(branch_names(&branches, Some("b")), ["b", "a"]);
    assert!(branch_names(&[branch("origin/x", BranchKind::Remote)], Some("x")).starts_with(&["x".to_string()]));
}
