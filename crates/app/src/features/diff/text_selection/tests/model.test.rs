use super::*;

#[test]
fn word_bounds() {
    assert_eq!(word_at("let foo_bar = 1;", 6), 4..11);
    assert_eq!(word_at("let foo_bar = 1;", 11), 11..12);
    assert_eq!(word_at("ab", 2), 0..2);
    assert_eq!(word_at("", 0), 0..0);
}

#[test]
fn cols() {
    let sel = TextSel {
        anchor: TextPos { line: 3, col: 4 },
        head: TextPos { line: 1, col: 2 },
        side: TextSide::Unified,
    };
    assert_eq!(sel.cols_in(1, TextSide::Unified, 10), Some(2..10));
    assert_eq!(sel.cols_in(2, TextSide::Unified, 10), Some(0..10));
    assert_eq!(sel.cols_in(3, TextSide::Unified, 10), Some(0..4));
    assert_eq!(sel.cols_in(3, TextSide::Left, 10), None);
    assert_eq!(sel.cols_in(4, TextSide::Unified, 10), None);
}
