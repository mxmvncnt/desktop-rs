use super::*;

#[test]
fn labels_follow_gitlab() {
    assert_eq!(look(Status::Success, false).label, "passed");
    assert_eq!(look(Status::Success, true).label, "warning");
    assert_eq!(look(Status::Failed, false).label, "failed");
    assert_eq!(look(Status::Canceled, false).label, "canceled");
    assert_eq!(look(Status::Manual, false).label, "manual");
}

#[test]
fn tones() {
    assert_eq!(look(Status::Success, false).tone, Tone::Success);
    assert_eq!(look(Status::Success, true).tone, Tone::Warning);
    assert_eq!(look(Status::Failed, false).tone, Tone::Danger);
    assert_eq!(look(Status::Running, false).tone, Tone::Running);
    assert_eq!(look(Status::Pending, false).tone, Tone::Warning);
    assert_eq!(look(Status::Skipped, false).tone, Tone::Muted);
}

#[test]
fn warnings_only_change_a_passed_status() {
    assert_eq!(look(Status::Failed, true), look(Status::Failed, false));
}
