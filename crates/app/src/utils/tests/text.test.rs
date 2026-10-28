use super::*;

#[test]
fn formats() {
    assert_eq!(format_ago(-5, Coarsest::Year), "just now");
    assert_eq!(format_ago(60, Coarsest::Year), "1 minute ago");
    assert_eq!(format_ago(7200, Coarsest::Year), "2 hours ago");
    assert_eq!(format_ago(86_400 * 45, Coarsest::Day), "45 days ago");
    assert_eq!(format_ago(86_400 * 45, Coarsest::Year), "1 month ago");
    assert_eq!(format_ago(31_536_000 * 3, Coarsest::Year), "3 years ago");
}
