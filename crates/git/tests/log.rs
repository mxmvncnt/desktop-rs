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
