mod common;

use gd_git::{Remote, Repository};

fn scratch() -> (tempfile::TempDir, Repository) {
    let tmp = common::tempdir();
    let repo = Repository::init(tmp.path().join("repo")).unwrap();
    (tmp, repo)
}

#[test]
fn remotes() {
    let (_tmp, repo) = scratch();
    assert!(repo.remote_list().unwrap().is_empty());
    repo.add_remote("origin", "https://example.com/a/b.git").unwrap();
    repo.add_remote("up", "git@example.com:u/b.git").unwrap();
    assert!(repo.add_remote("origin", "x").is_err());

    repo.set_remote_url("up", "git@example.com:u/c.git").unwrap();
    repo.set_remote_push_url("up", "git@example.com:push/c.git").unwrap();
    let list = repo.remote_list().unwrap();
    assert_eq!(list[0], Remote { name: "origin".into(), fetch_url: "https://example.com/a/b.git".into(), push_url: "https://example.com/a/b.git".into() });
    assert_eq!((list[1].fetch_url.as_str(), list[1].push_url.as_str()), ("git@example.com:u/c.git", "git@example.com:push/c.git"));
    assert!(list[1].has_push_url() && !list[0].has_push_url());

    repo.set_remote_push_url("up", "").unwrap();
    repo.set_remote_push_url("up", "").unwrap(); // already unset
    assert!(!repo.remote_list().unwrap()[1].has_push_url());

    repo.rename_remote("up", "upstream").unwrap();
    assert_eq!(repo.remote_list().unwrap()[1].name, "upstream");
    assert_eq!(repo.info().unwrap().default_remote.as_deref(), Some("origin"));
    repo.remove_remote("origin").unwrap();
    assert_eq!(repo.info().unwrap().default_remote.as_deref(), Some("upstream"));
    repo.remove_remote("upstream").unwrap();
    assert!(repo.remote_list().unwrap().is_empty());
}

#[test]
fn info_of_a_fresh_repository() {
    let (_tmp, repo) = scratch();
    let info = repo.info().unwrap();
    assert_eq!((info.head, info.default_remote), (None, None));
}
