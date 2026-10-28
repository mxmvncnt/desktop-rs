//! Typed rows of an app context menu. Features build `Vec<MenuRow<A>>` with
//! their own action enum `A`; `Menu::new` binds them to what they do and
//! `MenuView` renders them.

use gd_ui::Icon;

#[derive(Clone)]
pub enum MenuRow<A> {
    Item(RowItem<A>),
    Separator,
}

#[derive(Clone)]
pub struct RowItem<A> {
    pub action: A,
    pub label: String,
    pub icon: Icon,
    pub danger: bool,
    pub disabled: bool,
}

impl<A> RowItem<A> {
    pub fn new(action: A, label: impl Into<String>, icon: Icon) -> Self {
        Self { action, label: label.into(), icon, danger: false, disabled: false }
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<A: Clone> MenuRow<A> {
    pub fn item(action: A, label: impl Into<String>, icon: Icon) -> Self {
        Self::Item(RowItem::new(action, label, icon))
    }

    /// Keyboard navigation skips separators and disabled items.
    pub fn is_selectable(&self) -> bool {
        matches!(self, Self::Item(i) if !i.disabled)
    }

    /// The action Enter would run on this row.
    pub fn action(&self) -> Option<A> {
        match self {
            Self::Item(i) if !i.disabled => Some(i.action.clone()),
            _ => None,
        }
    }
}
