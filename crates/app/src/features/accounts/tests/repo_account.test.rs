use std::path::Path;

use super::*;
use crate::features::accounts::model::Provider;

fn acct(id: &str, host: &str) -> Account {
    Account { id: id.into(), provider: Provider::Generic, name: id.into(), host: host.into(), username: String::new(), oauth: false, token: String::new() }
}

#[test]
fn matches_the_remote_host_and_takes_the_first() {
    let list = [acct("a", "git.corp:8443"), acct("b", "github.com"), acct("c", "github.com")];
    let none = RepoAccounts::new();
    assert_eq!(account_for_repo(&list, &none, Path::new("/r"), "GitHub.com").unwrap().id, "b");
    assert_eq!(account_for_repo(&list, &none, Path::new("/r"), "git.corp").unwrap().id, "a");
    assert!(account_for_repo(&list, &none, Path::new("/r"), "gitlab.com").is_none());
}

#[test]
fn a_pick_wins_unless_its_account_is_gone() {
    let list = [acct("a", "github.com"), acct("b", "gitlab.com")];
    let mut picks = RepoAccounts::from([("/r".to_string(), "b".to_string())]);
    assert_eq!(account_for_repo(&list, &picks, Path::new("/r"), "github.com").unwrap().id, "b");
    assert_eq!(account_for_repo(&list, &picks, Path::new("/other"), "github.com").unwrap().id, "a");
    picks.insert("/r".into(), "gone".into());
    assert_eq!(account_for_repo(&list, &picks, Path::new("/r"), "github.com").unwrap().id, "a");
}
