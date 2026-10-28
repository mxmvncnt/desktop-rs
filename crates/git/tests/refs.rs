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

#[test]
fn remotes_tags_unpushed_and_base_branch() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    sh(&dir, &["checkout", "-q", "-b", "main"]);
    assert!(repo.remotes().unwrap().is_empty());
    assert!(repo.unpushed_commits().unwrap().is_empty());
    let commit = |msg: &str| {
        std::fs::write(dir.join("f"), msg).unwrap();
        sh(&dir, &["add", "."]);
        sh(&dir, &["commit", "-qm", msg]);
        sh(&dir, &["rev-parse", "HEAD"])
    };
    let first = commit("one");
    sh(&dir, &["remote", "add", "origin", "https://github.com/o/r.git"]);
    sh(&dir, &["remote", "add", "other", "https://example.com/x.git"]);
    assert_eq!(repo.remotes().unwrap()[0], ("origin".to_string(), "https://github.com/o/r.git".to_string()));
    assert_eq!(repo.tracking_remote_url().as_deref(), Some("https://github.com/o/r.git"));
    assert_eq!(repo.github_web_url().as_deref(), Some("https://github.com/o/r"));
    // No remote-tracking branch yet: everything is unpushed.
    assert_eq!(repo.unpushed_commits().unwrap().len(), 1);

    sh(&dir, &["update-ref", "refs/remotes/origin/main", &first]);
    sh(&dir, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    assert!(repo.unpushed_commits().unwrap().is_empty());
    assert_eq!(repo.default_base_branch().as_deref(), Some("origin/main"));
    sh(&dir, &["branch", "--set-upstream-to=origin/main"]);
    let second = commit("two");
    let unpushed = repo.unpushed_commits().unwrap();
    assert!(unpushed.len() == 1 && unpushed.contains(&second));

    sh(&dir, &["tag", "light", &first]);
    sh(&dir, &["tag", "-a", "-m", "m", "annotated", &second]);
    let tags = repo.tags_by_commit().unwrap();
    assert_eq!(tags[&first], ["light"]);
    assert_eq!(tags[&second], ["annotated"]);
}
