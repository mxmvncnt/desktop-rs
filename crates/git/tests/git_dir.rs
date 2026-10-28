mod common;

use common::sh;
use gd_git::*;

#[test]
fn open_from_subdir_and_linked_worktree() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    std::fs::create_dir(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/f"), "f\n").unwrap();
    sh(&dir, &["add", "."]);
    sh(&dir, &["commit", "-qm", "init"]);

    let from_sub = Repository::open(dir.join("sub")).unwrap();
    assert_eq!(from_sub.root(), repo.root());
    assert_eq!(repo.git_dir(), repo.common_git_dir());
    assert!(repo.git_dir().ends_with(".git"));

    let linked = tmp.path().join("linked");
    sh(&dir, &["worktree", "add", "-q", "-b", "wt", linked.to_str().unwrap()]);
    let wt = Repository::open(&linked).unwrap();
    assert!(wt.git_dir().ends_with(".git/worktrees/linked"));
    assert!(wt.common_git_dir().ends_with(".git"));
}
