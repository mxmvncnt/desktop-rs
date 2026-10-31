use super::*;

#[test]
fn formats_by_magnitude() {
    assert_eq!(format_duration(0), "0s");
    assert_eq!(format_duration(45), "45s");
    assert_eq!(format_duration(125), "2m 05s");
    assert_eq!(format_duration(3720), "1h 02m");
}

#[test]
fn unknown_without_times() {
    assert_eq!(duration_text(None, None), "—");
    assert_eq!(duration_text(Some(61), None), "1m 01s");
}
