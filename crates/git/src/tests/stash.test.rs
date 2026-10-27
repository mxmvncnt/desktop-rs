use super::parse_subject;

#[test]
fn subjects() {
    assert_eq!(parse_subject("WIP on main: abc123 Fix"), (Some("main".into()), String::new()));
    assert_eq!(parse_subject("On feat/x: my msg: more"), (Some("feat/x".into()), "my msg: more".into()));
    assert_eq!(parse_subject("WIP on (no branch): abc Fix"), (None, String::new()));
    assert_eq!(parse_subject("custom"), (None, "custom".into()));
}
