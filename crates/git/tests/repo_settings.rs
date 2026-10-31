mod common;

use gd_git::Repository;

#[test]
fn info_of_a_fresh_repository() {
    let tmp = common::tempdir();
    let repo = Repository::init(tmp.path().join("repo")).unwrap();
    let info = repo.info().unwrap();
    assert_eq!((info.head, info.default_remote), (None, None));
}
