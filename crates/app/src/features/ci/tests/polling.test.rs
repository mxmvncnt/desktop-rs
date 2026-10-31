use super::*;

#[test]
fn polls_faster_while_something_runs() {
    assert_eq!(poll_interval(true, false), Duration::from_secs(10));
    assert_eq!(poll_interval(false, false), Duration::from_secs(60));
}

#[test]
fn follows_a_running_log_every_five_seconds() {
    assert_eq!(poll_interval(true, true), Duration::from_secs(5));
    assert_eq!(poll_interval(false, true), Duration::from_secs(60));
}
