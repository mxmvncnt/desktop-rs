mod common;

use common::sh;
use gd_git::*;

#[test]
fn log_pages_and_reads_trailers() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    assert!(repo.log(0, 10).unwrap().is_empty());
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    for i in 1..=5 {
        std::fs::write(dir.join("f"), format!("{i}\n")).unwrap();
        sh(&dir, &["add", "."]);
        let msg = format!("c{i}\n\nbody {i}\n\nCo-authored-by: Al <al@x.io>");
        sh(&dir, &["commit", "-qm", &msg]);
    }
    let all = repo.log(0, 100).unwrap();
    let summaries: Vec<_> = all.iter().map(|c| c.summary.as_str()).collect();
    assert_eq!(summaries, ["c5", "c4", "c3", "c2", "c1"]);
    let page = repo.log(1, 2).unwrap();
    assert_eq!(page.iter().map(|c| c.summary.as_str()).collect::<Vec<_>>(), ["c4", "c3"]);
    let c = &page[0];
    assert_eq!((c.author_name.as_str(), c.author_email.as_str()), ("T", "t@x.io"));
    assert_eq!(c.parents, [all[2].sha.clone()]);
    assert_eq!(c.body, "body 4\n\nCo-authored-by: Al <al@x.io>");
    assert_eq!(c.co_authors, [CoAuthor { name: "Al".into(), email: "al@x.io".into() }]);
    assert!(c.sha.starts_with(&c.short_sha));
}

#[test]
fn tree_changes_for_commit_and_comparison() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    sh(&dir, &["checkout", "-q", "-b", "main"]);
    for f in ["old", "gone", "keep"] {
        std::fs::write(dir.join(f), format!("{f}\nline\nline\nline\n")).unwrap();
    }
    sh(&dir, &["add", "."]);
    sh(&dir, &["commit", "-qm", "root"]);
    let root = repo.log(0, 1).unwrap().remove(0);
    let files: Vec<_> = repo.commit_files(&root.sha).unwrap().into_iter().map(|f| (f.path, f.status)).collect();
    assert_eq!(files, [("gone".into(), FileStatus::New), ("keep".into(), FileStatus::New), ("old".into(), FileStatus::New)]);

    sh(&dir, &["checkout", "-q", "-b", "feat"]);
    sh(&dir, &["mv", "old", "new"]);
    sh(&dir, &["rm", "-q", "gone"]);
    std::fs::write(dir.join("keep"), "changed\n").unwrap();
    sh(&dir, &["add", "-A"]);
    sh(&dir, &["commit", "-qm", "feat"]);
    let got: Vec<_> = repo.commit_files("HEAD").unwrap().into_iter().map(|f| (f.path, f.old_path, f.status)).collect();
    assert_eq!(
        got,
        [
            ("gone".into(), None, FileStatus::Deleted),
            ("keep".into(), None, FileStatus::Modified),
            ("new".into(), Some("old".into()), FileStatus::Renamed),
        ]
    );
    let cmp = repo.compare("main", "feat").unwrap();
    assert_eq!((cmp.ahead, cmp.behind, cmp.files.len()), (1, 0, 3));
    assert_eq!(cmp.merge_base, root.sha);
}

#[test]
fn blob_sides_read_missing_paths_as_empty() {
    let tmp = common::tempdir();
    let dir = tmp.path().join("repo");
    let repo = Repository::init(&dir).unwrap();
    sh(&dir, &["config", "user.email", "t@x.io"]);
    sh(&dir, &["config", "user.name", "T"]);
    std::fs::write(dir.join("f"), "one\ntwo\n").unwrap();
    sh(&dir, &["add", "."]);
    sh(&dir, &["commit", "-qm", "root"]);
    let root = repo.log(0, 1).unwrap().remove(0);
    let file = repo.commit_files(&root.sha).unwrap().remove(0);
    let diff = repo.commit_diff(&root.sha, &file, Default::default()).unwrap();
    assert_eq!((diff.added, diff.deleted), (2, 0));
    let blob = |rev: &str, path: &str| repo.read_side(&Side::Blob { rev: rev.into(), path: path.into() }).unwrap();
    assert_eq!(blob("HEAD", "f"), b"one\ntwo\n");
    assert!(blob("HEAD", "missing").is_empty());
}
