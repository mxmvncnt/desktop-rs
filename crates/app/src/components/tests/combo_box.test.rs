use super::*;

#[test]
fn enter_applies_the_exact_entry_then_the_first_then_the_text_and_nothing_for_an_empty_text() {
    let values = ["adam".to_string(), "ada".to_string()];
    assert_eq!(enter_choice(&values, "ADA"), "ada");
    assert_eq!(enter_choice(&values, "ad"), "adam");
    assert_eq!(enter_choice(&[], " v1.0 "), "v1.0");
    assert_eq!(enter_choice(&values, "  "), "");
}
