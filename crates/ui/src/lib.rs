//! `gd-ui` — zeron-style design system for desktop-rs.
//!
//! - [`theme`] / [`themes`]: tokens (colors, sizes, fonts) — `Theme::of(cx)`.
//! - [`motion`]: the zeron animation catalog (fade-in, menu-in, …).
//! - `components/`: one primitive per file (buttons, checkbox, tabs, menus,
//!   dialogs, …), all re-exported here.
//! - [`assets`]: embedded icons + fonts.

pub mod assets;
mod components;
pub mod motion;
pub mod theme;
pub mod themes;

pub use assets::{Assets, Icon};
pub use components::*;
pub use theme::Theme;

use gpui::App;

/// Install fonts and the theme global. Call once at startup.
pub fn init(cx: &mut App) {
    assets::load_fonts(cx);
    cx.set_global(Theme::for_family(themes::DEFAULT_THEME, false));
}
