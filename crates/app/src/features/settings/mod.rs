//! Settings modal (appearance and diff pages) and the persisted `Settings`
//! model (`model.rs`). Shown as `Overlay::Settings`.

mod about_page;
mod appearance_page;
mod diff_page;
mod handlers;
mod model;
mod settings_modal;
#[cfg(feature = "snapshot")]
pub mod snapshot;
mod state;

pub use model::{Settings, ThemeMode, config_dir, themes_dir};
pub use settings_modal::SettingsModal;
pub use state::SettingsPage;
