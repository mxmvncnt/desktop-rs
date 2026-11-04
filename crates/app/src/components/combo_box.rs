//! A text field that is also a dropdown (`ComboMenu`): clicking it lists the choices, typing narrows them, and a pick or Enter applies one
//! (typed text alone never does; emptying the field applies ""). Closing it without a pick puts the applied value back.
//! The owner renders the entity (it fills its parent's width), listens to `ComboPicked`, and calls `set_value` when the value changes elsewhere.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Bounds, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, SharedString, Styled, Subscription, Window, canvas, div,
};
use gpui_elements::editable_text::{
    EditableTextState, StringStorage, TextChanged,
    actions::{Enter, Escape},
};

use super::combo_menu::ComboMenu;
use super::text_field;

/// A line of the dropdown: the value that goes in the field and a faint note next to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboEntry {
    pub value: String,
    pub detail: Option<String>,
}

/// What the dropdown lists for a text ("" while nothing narrows it).
type Entries = Box<dyn Fn(&str, &gpui::App) -> Vec<ComboEntry>>;

/// A value was applied: picked from the list, by Enter, or the field was emptied.
pub struct ComboPicked(pub String);

pub struct ComboBox {
    id: &'static str,
    placeholder: SharedString,
    text: Entity<EditableTextState>,
    /// What is applied: shown while the dropdown is closed.
    value: String,
    open: bool,
    /// Text was typed since the dropdown opened: it narrows the list, and Enter applies the first entry.
    typed: bool,
    entries: Entries,
    /// The field in window coordinates, set while painting it: the dropdown opens below it.
    field: Rc<Cell<Bounds<Pixels>>>,
    _edits: Subscription,
}

impl EventEmitter<ComboPicked> for ComboBox {}

impl ComboBox {
    pub fn new(
        id: &'static str,
        placeholder: impl Into<SharedString>,
        entries: impl Fn(&str, &gpui::App) -> Vec<ComboEntry> + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        let text = cx.new(|cx| EditableTextState::new(StringStorage::default(), cx));
        let _edits = cx.subscribe(&text, |this, _, _: &TextChanged, cx| this.edited(cx));
        Self {
            id,
            placeholder: placeholder.into(),
            text,
            value: String::new(),
            open: false,
            typed: false,
            entries: Box::new(entries),
            field: Rc::default(),
            _edits,
        }
    }

    /// Show `value` as the applied one and close the dropdown.
    pub fn set_value(&mut self, value: &str, cx: &mut Context<Self>) {
        self.value = value.to_string();
        self.open = false;
        self.typed = false;
        self.text.update(cx, |s, cx| s.emplace(value, cx));
        cx.notify();
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// List all the choices.
    pub fn open(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            self.open = true;
            self.typed = false;
            cx.notify();
        }
    }

    /// Put `text` in the field as if it was typed.
    #[cfg(feature = "snapshot")]
    pub fn type_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.text.update(cx, |s, cx| s.emplace(text, cx));
    }

    /// The text changed. The app only changes it to the applied value, so anything else was typed.
    fn edited(&mut self, cx: &mut Context<Self>) {
        let text = self.text.read(cx).as_str().trim().to_string();
        if text == self.value {
            return;
        }
        if text.is_empty() {
            self.value.clear();
            cx.emit(ComboPicked(String::new()));
        }
        self.open = true;
        self.typed = !text.is_empty();
        cx.notify();
    }

    pub(super) fn pick(&mut self, value: String, cx: &mut Context<Self>) {
        self.set_value(&value, cx);
        cx.emit(ComboPicked(value));
    }

    /// Esc or a click outside the dropdown: close it, back to the applied value.
    pub(super) fn dismiss(&mut self, cx: &mut Context<Self>) {
        let value = self.value.clone();
        self.set_value(&value, cx);
    }

    /// The typed text that narrows the list, "" when nothing does.
    fn query(&self, cx: &gpui::App) -> String {
        if self.typed {
            self.text.read(cx).as_str().trim().to_string()
        } else {
            String::new()
        }
    }
}

impl Render for ComboBox {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.open.then(|| ComboMenu {
            combo: cx.entity(),
            id: self.id,
            entries: (self.entries)(&self.query(cx), cx),
            typed: self.typed,
            field: self.field.get(),
        });
        let enter = cx.listener(|this, _: &Enter, _, cx| {
            if !this.typed {
                return;
            }
            cx.stop_propagation();
            let values: Vec<String> = (this.entries)(&this.query(cx), cx)
                .into_iter()
                .map(|e| e.value)
                .collect();
            let choice = enter_choice(&values, this.text.read(cx).as_str());
            this.pick(choice, cx);
        });
        let escape = cx.listener(|this, _: &Escape, _, cx| {
            if this.open {
                cx.stop_propagation();
                this.dismiss(cx);
            }
        });
        let field = self.field.clone();
        div()
            .w_full()
            .capture_action(enter)
            .capture_action(escape)
            .child(
                div()
                    .relative()
                    // The input handles its own clicks (caret, focus) and stops them, so this listens before it, in the capture phase.
                    .capture_any_mouse_down(cx.listener(|this, _, _, cx| this.open(cx)))
                    .child(
                        canvas(move |bounds, _, _| field.set(bounds), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    )
                    .child(text_field(self.id, &self.text, &self.placeholder, cx))
                    .children(menu),
            )
    }
}

/// What Enter applies: nothing for an empty text (it clears the field), the entry that is the text, else the first entry, else the text as
/// typed (a value not listed).
pub fn enter_choice(values: &[String], typed: &str) -> String {
    let typed = typed.trim();
    if typed.is_empty() {
        return String::new();
    }
    values
        .iter()
        .find(|v| v.eq_ignore_ascii_case(typed))
        .or(values.first())
        .cloned()
        .unwrap_or_else(|| typed.to_string())
}

#[cfg(test)]
#[path = "tests/combo_box.test.rs"]
mod tests;
