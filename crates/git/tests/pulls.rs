//! A pull request as a forge keeps it: the head under `refs/pull/N/head` of a bare remote.

use std::path::Path;

mod common;

use common::sh;
use gd_git::*;

fn commit(dir: &Path, file: &str, content: &str, msg: &str) {
    std::fs::write(dir.join(file), content).unwrap();
    sh(dir, &["add", "-A"]);
    sh(dir, &["commit", "-q", "-m", msg]);
}

#[test]
fn fetch_and_checkout_a_pull_request() {
    let tmp = common::tempdir();
    let base = tmp.path().to_path_buf();
    let remote = base.join("remote.git");
    let author = base.join("author");
    let work = base.join("work");
    sh(&base, &["init", "-q", "--bare", "-b", "main", "remote.git"]);
    sh(&base, &["clone", "-q", remote.to_str().unwrap(), "author"]);
    for (k, v) in [("user.name", "Tester"), ("user.email", "t@example.com")] {
        sh(&author, &["config", k, v]);
    }
    sh(&author, &["checkout", "-q", "-b", "main"]);
    commit(&author, "a.txt", "one\n", "base");
    sh(&author, &["push", "-q", "origin", "main"]);
    sh(&author, &["checkout", "-q", "-b", "topic"]);
    commit(&author, "a.txt", "one\ntwo\n", "add two");
    commit(&author, "b.txt", "new\n", "add b");
    // The forge's ref, and a branch for the same-repository case.
    sh(
        &author,
        &["push", "-q", "origin", "topic:refs/pull/7/head", "topic"],
    );
    // main moves on after the branch point.
    sh(&author, &["checkout", "-q", "main"]);
    commit(&author, "c.txt", "later\n", "later on main");
    sh(&author, &["push", "-q", "origin", "main"]);

    sh(&base, &["clone", "-q", remote.to_str().unwrap(), "work"]);
    let repo = Repository::open(&work).unwrap();
    let head = repo
        .fetch_pull("refs/pull/7/head", "main", Some("topic"))
        .unwrap();
    assert_eq!(
        head.commits
            .iter()
            .map(|c| c.summary.as_str())
            .collect::<Vec<_>>(),
        ["add two", "add b"]
    );
    let paths: Vec<_> = head.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        ["a.txt", "b.txt"],
        "merge-base diff: main's later commit is not in it"
    );
    assert_eq!(head.head, sh(&work, &["rev-parse", "refs/pull/7/head"]));

    // A fork: a branch at the head, no upstream.
    repo.checkout_pull(&PullCheckout {
        pull_ref: "refs/pull/7/head".into(),
        branch: "pr/7".into(),
        upstream: None,
    })
    .unwrap();
    assert_eq!(sh(&work, &["rev-parse", "HEAD"]), head.head);
    // The same repository: a branch tracking the head branch.
    sh(&work, &["checkout", "-q", "main"]);
    repo.checkout_pull(&PullCheckout {
        pull_ref: "refs/pull/7/head".into(),
        branch: "topic".into(),
        upstream: Some("origin/topic".into()),
    })
    .unwrap();
    assert_eq!(
        sh(&work, &["rev-parse", "--abbrev-ref", "@{upstream}"]),
        "origin/topic"
    );
    // Checking it out again with a local commit on it keeps the commit.
    for (k, v) in [("user.name", "Tester"), ("user.email", "t@example.com")] {
        sh(&work, &["config", k, v]);
    }
    commit(&work, "d.txt", "local\n", "local work");
    let local = sh(&work, &["rev-parse", "HEAD"]);
    sh(&work, &["checkout", "-q", "main"]);
    repo.checkout_pull(&PullCheckout {
        pull_ref: "refs/pull/7/head".into(),
        branch: "topic".into(),
        upstream: Some("origin/topic".into()),
    })
    .unwrap();
    assert_eq!(sh(&work, &["rev-parse", "HEAD"]), local);
}
