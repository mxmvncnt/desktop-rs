//! Settings modal (appearance and diff pages) and the persisted `Settings`
//! model (`model.rs`). Shown as `Overlay::Settings`.

mod appearance_page;
mod diff_page;
mod handlers;
mod model;
mod settings_modal;
mod settings_nav;
mod state;
#[cfg(feature = "snapshot")]
pub mod snapshot;

pub use model::{Settings, ThemeMode, config_dir};
pub use settings_modal::SettingsModal;
pub use state::SettingsPage;
