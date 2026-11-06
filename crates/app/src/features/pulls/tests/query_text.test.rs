use super::*;

#[test]
fn tabs_replace_the_state_terms() {
    let q = "is:pr state:open sort:updated-desc (involves:@me OR is:open) -author:@me";
    assert_eq!(
        with_state(q, Some(PullState::Merged)),
        "is:pr is:merged sort:updated-desc (involves:@me OR is:open) -author:@me",
        "a state inside parentheses stays"
    );
    assert_eq!(
        with_state("label:x is:merged", Some(PullState::Closed)),
        "state:closed is:unmerged label:x"
    );
    assert_eq!(with_state(DEFAULT_QUERY, None), "is:pr");
}
