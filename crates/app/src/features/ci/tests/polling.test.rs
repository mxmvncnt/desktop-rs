use super::*;

#[test]
fn polls_faster_while_something_runs() {
    assert_eq!(poll_interval(true), Duration::from_secs(10));
    assert_eq!(poll_interval(false), Duration::from_secs(60));
}
