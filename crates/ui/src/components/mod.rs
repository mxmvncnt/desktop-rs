//! gd-ui components, one per file. Everything is re-exported at the crate root.

mod button;
mod checkbox;
mod context_menu;
mod dialog;
mod resize_handle;
mod surface;
mod tabs;
mod toolbar_button;
mod tooltip;

pub use button::{Button, ButtonStyle, icon_button};
pub use checkbox::{CheckState, Checkbox};
pub use context_menu::{ContextMenu, MenuEntry, MenuItem};
pub use dialog::Dialog;
pub use resize_handle::ResizeHandle;
pub use surface::{badge, divider, icon, overlay_card, pressable, vdivider};
pub use tabs::Tabs;
pub use toolbar_button::ToolbarButton;
pub use tooltip::tooltip;

type ClickHandler = Box<dyn Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>;
