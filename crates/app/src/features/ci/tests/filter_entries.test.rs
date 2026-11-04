use super::*;

fn branch(name: &str, kind: BranchKind) -> Branch {
    Branch {
        name: name.into(),
        kind,
        is_head: false,
        upstream: None,
        ahead_behind: None,
        tip_time: 0,
    }
}

fn values(entries: &[Entry]) -> Vec<&str> {
    entries.iter().map(|e| e.value.as_str()).collect()
}

fn user(login: &str, name: Option<&str>) -> User {
    User {
        name: name.map(str::to_string),
        username: login.into(),
        avatar_url: None,
    }
}

#[test]
fn remote_branches_that_are_local_are_listed_once_and_head_is_skipped() {
    let branches = [
        branch("main", BranchKind::Local),
        branch("feat/ci", BranchKind::Local),
        branch("origin/HEAD", BranchKind::Remote),
        branch("origin/main", BranchKind::Remote),
        branch("origin/release/1.5", BranchKind::Remote),
        branch("upstream/main", BranchKind::Remote),
    ];
    assert_eq!(
        values(&branch_entries(&branches, None, "")),
        ["main", "feat/ci", "release/1.5"]
    );
}

#[test]
fn the_current_branch_comes_first_and_is_marked() {
    let branches = [
        branch("a", BranchKind::Local),
        branch("b", BranchKind::Local),
        branch("origin/c", BranchKind::Remote),
    ];
    let entries = branch_entries(&branches, Some("b"), "");
    assert_eq!(values(&entries), ["b", "a", "c"]);
    assert_eq!(entries[0].detail.as_deref(), Some("current"));
    assert_eq!(entries[1].detail, None);
    // a current branch that is only on the remote leads too
    assert_eq!(
        values(&branch_entries(
            &[
                branch("origin/x", BranchKind::Remote),
                branch("y", BranchKind::Local)
            ],
            Some("x"),
            ""
        )),
        ["x", "y"]
    );
}

#[test]
fn typing_narrows_the_branches_by_a_part_of_the_name_and_keeps_the_current_one_first() {
    let branches = [
        branch("main", BranchKind::Local),
        branch("feat/release-notes", BranchKind::Local),
        branch("release/1.5", BranchKind::Local),
        branch("origin/RELEASE/1.4", BranchKind::Remote),
    ];
    assert_eq!(
        values(&branch_entries(&branches, Some("release/1.5"), "rel")),
        ["release/1.5", "feat/release-notes", "RELEASE/1.4"]
    );
    assert_eq!(
        values(&branch_entries(&branches, Some("main"), "rel")),
        ["feat/release-notes", "release/1.5", "RELEASE/1.4"]
    );
    assert!(branch_entries(&branches, None, "zzz").is_empty());
}

#[test]
fn users_match_a_part_of_the_login_or_of_the_name_ignoring_case() {
    let users = [
        user("ada", Some("Ada Lovelace")),
        user("linus", Some("Linus T.")),
        user("jdoe", Some("Grace Hopper")),
    ];
    assert_eq!(values(&user_entries(&users, "AD")), ["ada"]);
    assert_eq!(values(&user_entries(&users, "hopp")), ["jdoe"]);
    assert_eq!(values(&user_entries(&users, "n")), ["linus"]);
    assert!(user_entries(&users, "zzz").is_empty());
    assert_eq!(
        values(&user_entries(&users, "  ")),
        ["ada", "jdoe", "linus"]
    );
}

#[test]
fn prefix_matches_come_first_and_each_login_is_listed_once() {
    let users = [
        user("bob-ada", None),
        user("ada", None),
        user("Ada", None),
        user("adam", None),
    ];
    assert_eq!(
        values(&user_entries(&users, "ada")),
        ["ada", "adam", "bob-ada"]
    );
}

#[test]
fn enter_applies_the_exact_entry_then_the_first_then_the_text_and_nothing_for_an_empty_text() {
    let values = ["adam".to_string(), "ada".to_string()];
    assert_eq!(on_enter(&values, "ADA"), "ada");
    assert_eq!(on_enter(&values, "ad"), "adam");
    assert_eq!(on_enter(&[], " v1.0 "), "v1.0");
    assert_eq!(on_enter(&values, "  "), "");
}
