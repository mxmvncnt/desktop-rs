//! The one context menu: [`Menu`] is the open state (rows bound to what they
//! do, keyboard highlight, the single arrow-key loop) and [`MenuView`]
//! renders it with `gd_ui::ContextMenu`.

use std::rc::Rc;

use gd_ui::{ContextMenu, MenuEntry, MenuItem};
use gpui::{App, Context, ElementId, Entity, IntoElement, Pixels, Point, RenderOnce, Window};

use super::{MenuRow, RowItem};
use crate::workspace::Workspace;

/// Something a menu row or an overlay key does to the workspace.
pub type Run = Rc<dyn Fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>;

#[derive(Clone)]
pub struct Menu {
    pub position: Point<Pixels>,
    /// Keyboard highlight: index into the rows.
    pub selected: Option<usize>,
    rows: Vec<MenuRow<Run>>,
}

impl Menu {
    /// Menu of `rows` at `position`; choosing a row calls `run` with its action.
    pub fn new<A: Clone + 'static>(
        position: Point<Pixels>,
        rows: Vec<MenuRow<A>>,
        run: impl Fn(&mut Workspace, A, &mut Window, &mut Context<Workspace>) + 'static,
    ) -> Self {
        let run = Rc::new(run);
        let bind = |row: MenuRow<A>| match row {
            MenuRow::Separator => MenuRow::Separator,
            MenuRow::Item(i) => {
                let (run, action) = (run.clone(), i.action);
                let action: Run = Rc::new(move |ws, w, cx| run(ws, action.clone(), w, cx));
                MenuRow::Item(RowItem { action, label: i.label, icon: i.icon, danger: i.danger, disabled: i.disabled })
            }
        };
        Self { position, selected: None, rows: rows.into_iter().map(bind).collect() }
    }

    /// Arrow keys: move the highlight, skipping separators and disabled rows.
    pub fn step(&mut self, delta: isize) {
        let n = self.rows.len() as isize;
        let mut ix = self.selected.map_or(if delta > 0 { -1 } else { n }, |s| s as isize);
        for _ in 0..n {
            ix = (ix + delta).rem_euclid(n);
            if self.rows[ix as usize].is_selectable() {
                self.selected = Some(ix as usize);
                break;
            }
        }
    }

    /// Enter: what the highlighted row does, if it can be chosen.
    pub fn confirm(&self) -> Option<Run> {
        self.rows.get(self.selected?)?.action()
    }
}

#[derive(IntoElement)]
pub struct MenuView {
    /// Must change per open so the entrance animation restarts.
    pub id: ElementId,
    pub menu: Menu,
    pub ws: Entity<Workspace>,
    pub on_dismiss: Run,
}

impl RenderOnce for MenuView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (ws, dismiss) = (self.ws.clone(), self.on_dismiss);
        ContextMenu::new(self.id, self.menu.position)
            .selected(self.menu.selected)
            .on_dismiss(move |w, cx| ws.update(cx, |this, cx| dismiss(this, w, cx)))
            .entries(self.menu.rows.into_iter().map(|row| menu_entry(row, &self.ws)))
    }
}

fn menu_entry(row: MenuRow<Run>, ws: &Entity<Workspace>) -> MenuEntry {
    let MenuRow::Item(r) = row else { return MenuEntry::Separator };
    let (ws, run) = (ws.clone(), r.action);
    let item = MenuItem::new(r.label)
        .icon(r.icon)
        .disabled(r.disabled)
        .on_select(move |w, cx| ws.update(cx, |this, cx| run(this, w, cx)));
    MenuEntry::Item(if r.danger { item.danger() } else { item })
}
