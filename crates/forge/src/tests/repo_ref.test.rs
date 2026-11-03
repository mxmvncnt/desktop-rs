use super::*;

#[test]
fn from_remote() {
    for url in [
        "git@github.com:o/r.git",
        "https://github.com/o/r",
        "ssh://git@github.com:22/o/r.git",
    ] {
        let r = RepoRef::from_remote(url).unwrap();
        assert_eq!(
            (r.host.as_str(), r.owner.as_str(), r.name.as_str()),
            ("github.com", "o", "r"),
            "{url}"
        );
    }
    assert!(RepoRef::from_remote("https://github.com/only").is_none());
}

#[test]
fn gitlab_subgroups() {
    let r = RepoRef::from_remote("git@gitlab.example.com:group/sub/my-repo.git").unwrap();
    assert_eq!(r.owner, "group/sub");
    assert_eq!(r.full_path(), "group/sub/my-repo");
    assert_eq!(r.gitlab_id(), "group%2Fsub%2Fmy-repo");
}
