use gd_ui::Icon;
use gpui::point;

use super::*;

/// Rows: item, separator, disabled item, item.
fn menu() -> Menu {
    let rows = vec![
        MenuRow::item(0, "a", Icon::Check),
        MenuRow::Separator,
        MenuRow::Item(RowItem::new(1, "b", Icon::Check).disabled(true)),
        MenuRow::item(2, "c", Icon::Check),
    ];
    Menu::new(point(gpui::px(0.0), gpui::px(0.0)), rows, |_, _: i32, _, _| {})
}

fn stepped(menu: &mut Menu, deltas: &[isize]) -> Vec<Option<usize>> {
    deltas
        .iter()
        .map(|d| {
            menu.step(*d);
            menu.selected
        })
        .collect()
}

#[test]
fn down_skips_separators_and_disabled_rows_and_wraps() {
    assert_eq!(stepped(&mut menu(), &[1, 1, 1]), [Some(0), Some(3), Some(0)]);
}

#[test]
fn up_starts_at_the_last_row_and_wraps() {
    assert_eq!(stepped(&mut menu(), &[-1, -1, -1]), [Some(3), Some(0), Some(3)]);
}

#[test]
fn step_on_a_menu_without_selectable_rows_selects_nothing() {
    let rows = vec![MenuRow::Separator, MenuRow::Item(RowItem::new(0, "x", Icon::Check).disabled(true))];
    let mut menu = Menu::new(point(gpui::px(0.0), gpui::px(0.0)), rows, |_, _: i32, _, _| {});
    menu.step(1);
    assert_eq!(menu.selected, None);
}

#[test]
fn confirm_needs_a_selectable_highlighted_row() {
    let mut menu = menu();
    assert!(menu.confirm().is_none(), "nothing highlighted");
    menu.selected = Some(1);
    assert!(menu.confirm().is_none(), "separator");
    menu.selected = Some(2);
    assert!(menu.confirm().is_none(), "disabled row");
    menu.selected = Some(3);
    assert!(menu.confirm().is_some());
}
