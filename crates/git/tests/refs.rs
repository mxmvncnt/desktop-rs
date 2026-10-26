mod common;

use common::sh;
use gd_git::*;

#[test]
fn branches_local_then_remote_skip_symbolic_head() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    sh(&dir, &["checkout", "-q", "-b", "main"]);
    std::fs::write(dir.join("a"), "a\n").unwrap();
    sh(&dir, &["add", "."]);
    sh(&dir, &["commit", "-qm", "one",]);
    sh(&dir, &["branch", "old"]);
    std::fs::write(dir.join("a"), "b\n").unwrap();
    sh(&dir, &["commit", "-qam", "two"]);
    sh(&dir, &["remote", "add", "origin", "https://example.com/o/r.git"]);
    sh(&dir, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    sh(&dir, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    sh(&dir, &["config", "branch.main.remote", "origin"]);
    sh(&dir, &["config", "branch.main.merge", "refs/heads/main"]);

    let got: Vec<_> = repo.branches().unwrap().into_iter().map(|b| (b.name, b.kind, b.is_head, b.upstream)).collect();
    assert_eq!(
        got,
        [
            ("main".to_string(), BranchKind::Local, true, Some("origin/main".to_string())),
            ("old".to_string(), BranchKind::Local, false, None),
            ("origin/main".to_string(), BranchKind::Remote, false, None),
        ]
    );
}
