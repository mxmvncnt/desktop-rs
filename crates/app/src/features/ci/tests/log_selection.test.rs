use std::sync::Arc;

use gd_forge::parse_log;

use super::*;
use crate::features::ci::ops::visible_lines;

fn store(raw: &str, toggled: &[usize]) -> CiStore {
    let mut ci = CiStore::default();
    let log = parse_log(raw);
    ci.log.toggled = toggled.iter().copied().collect();
    ci.log.visible = visible_lines(&log, &ci.log.toggled);
    ci.log.log = Some(Arc::new(log));
    ci
}

fn select(ci: &mut CiStore, from: (usize, usize), to: (usize, usize)) {
    let pos = |(line, col)| TextPos { line, col };
    ci.log.sel = Some(TextSel {
        anchor: pos(from),
        head: pos(to),
        side: TextSide::Unified,
    });
}

#[test]
fn a_selection_over_lines_is_cut_at_both_ends() {
    let mut ci = store("alpha\nbeta\ngamma", &[]);
    select(&mut ci, (0, 2), (2, 3));
    assert_eq!(ci.log_selected_text(), "pha\nbeta\ngam");
}

#[test]
fn the_end_can_come_before_the_start() {
    let mut ci = store("alpha\nbeta", &[]);
    select(&mut ci, (1, 2), (0, 3));
    assert_eq!(ci.log_selected_text(), "ha\nbe");
}

#[test]
fn lines_hidden_by_a_collapsed_section_are_not_copied() {
    // GitHub groups start collapsed: line 2 is hidden.
    let mut ci = store("before\n##[group]Step\nhidden\n##[endgroup]\nafter", &[]);
    assert_eq!(ci.log.visible, [0, 1, 3]);
    select(&mut ci, (0, 0), (3, 5));
    assert_eq!(ci.log_selected_text(), "before\nStep\nafter");
}

#[test]
fn nothing_selected_copies_nothing() {
    let mut ci = store("alpha", &[]);
    assert_eq!(ci.log_selected_text(), "");
    select(&mut ci, (0, 2), (0, 2));
    assert_eq!(ci.log_selected_text(), "");
}
