use super::*;

fn user(name: &str) -> User {
    User {
        name: None,
        username: name.into(),
        avatar_url: None,
    }
}

#[test]
fn a_comment_does_not_undo_an_approval() {
    let reviews = latest_verdicts([
        (user("ada"), Verdict::ChangesRequested),
        (user("grace"), Verdict::Commented),
        (user("ada"), Verdict::Approved),
        (user("ada"), Verdict::Commented),
        (user("grace"), Verdict::Dismissed),
    ]);
    let got: Vec<_> = reviews
        .iter()
        .map(|r| (r.user.username.as_str(), r.verdict))
        .collect();
    assert_eq!(
        got,
        [("ada", Verdict::Approved), ("grace", Verdict::Dismissed)]
    );
}
