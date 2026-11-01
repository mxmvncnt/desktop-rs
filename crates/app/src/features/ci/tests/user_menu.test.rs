use super::*;

fn user(login: &str, name: Option<&str>) -> User {
    User { name: name.map(str::to_string), username: login.into(), avatar_url: None }
}

fn logins(s: &[Suggestion]) -> Vec<&str> {
    s.iter().map(|s| s.login.as_str()).collect()
}

#[test]
fn matches_a_part_of_the_login_or_of_the_name_ignoring_case() {
    let users = [user("ada", Some("Ada Lovelace")), user("linus", Some("Linus T.")), user("jdoe", Some("Grace Hopper"))];
    assert_eq!(logins(&suggest(&users, "AD")), ["ada"]);
    assert_eq!(logins(&suggest(&users, "hopp")), ["jdoe"]);
    assert_eq!(logins(&suggest(&users, "n")), ["linus"]);
    assert!(suggest(&users, "zzz").is_empty());
    assert!(suggest(&users, "  ").is_empty());
}

#[test]
fn prefix_matches_come_first_and_each_login_is_listed_once() {
    let users = [user("bob-ada", None), user("ada", None), user("Ada", None), user("adam", None)];
    assert_eq!(logins(&suggest(&users, "ada")), ["ada", "adam", "bob-ada"]);
}

#[test]
fn enter_prefers_the_exact_login_then_the_first_suggestion_then_the_text() {
    let suggestions = [Suggestion { login: "adam".into(), name: None }, Suggestion { login: "ada".into(), name: None }];
    assert_eq!(on_enter(&suggestions, "ADA"), "ada");
    assert_eq!(on_enter(&suggestions, "ad"), "adam");
    assert_eq!(on_enter(&[], " someone "), "someone");
}
