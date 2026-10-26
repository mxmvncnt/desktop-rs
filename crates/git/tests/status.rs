mod common;

use common::sh;
use gd_git::*;

#[test]
fn status_kinds_branch_and_upstream() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    sh(&dir, &["checkout", "-q", "-b", "main"]);
    let unborn = repo.status().unwrap();
    assert_eq!((unborn.branch.as_deref(), unborn.head_oid), (Some("main"), None));

    for f in ["keep", "gone", "old"] {
        std::fs::write(dir.join(f), format!("{f} content\nline\nline\nline\n")).unwrap();
    }
    sh(&dir, &["add", "."]);
    sh(&dir, &["commit", "-qm", "init"]);
    std::fs::write(dir.join("keep"), "changed\n").unwrap();
    std::fs::remove_file(dir.join("gone")).unwrap();
    sh(&dir, &["mv", "old", "new"]);
    std::fs::create_dir(dir.join("d")).unwrap();
    std::fs::write(dir.join("d/u"), "u\n").unwrap();
    std::fs::write(dir.join("added"), "a\n").unwrap();
    sh(&dir, &["add", "added"]);

    let st = repo.status().unwrap();
    let got: Vec<_> = st.files.iter().map(|f| (f.path.as_str(), f.old_path.as_deref(), f.status)).collect();
    assert_eq!(
        got,
        [
            ("added", None, FileStatus::New),
            ("d/u", None, FileStatus::Untracked),
            ("gone", None, FileStatus::Deleted),
            ("keep", None, FileStatus::Modified),
            ("new", Some("old"), FileStatus::Renamed),
        ]
    );
    assert!(st.head_oid.is_some() && st.upstream.is_none());

    sh(&dir, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    sh(&dir, &["remote", "add", "origin", "https://example.com/o/r.git"]);
    sh(&dir, &["config", "branch.main.remote", "origin"]);
    sh(&dir, &["config", "branch.main.merge", "refs/heads/main"]);
    sh(&dir, &["add", "-A"]);
    sh(&dir, &["commit", "-qm", "two"]);
    let st = repo.status().unwrap();
    assert_eq!(st.upstream.as_deref(), Some("origin/main"));
    assert_eq!(st.ahead_behind, AheadBehind { ahead: 1, behind: 0 });
}
