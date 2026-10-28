//! Display options of the diff: toolbar button, popover menu, "whitespace hidden" note.

mod button;
mod menu;
mod whitespace_note;

pub use button::DiffOptionsButton;
pub use menu::DiffOptionsMenu;
pub(super) use whitespace_note::WhitespaceNote;
