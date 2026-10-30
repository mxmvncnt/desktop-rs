//! Persisted app state: one `settings.toml` in `$GD_CONFIG_DIR` (override, used by
//! dev/test runs so nothing is written outside the project), else `desktop-rs`
//! under the platform config dir (`dirs::config_dir`).

use std::path::PathBuf;

use gd_ui::themes::ColorThemes;
use serde::{Deserialize, Serialize};

pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("GD_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    Some(dirs::config_dir()?.join("desktop-rs"))
}

/// Where user color themes (`*.json`) live.
pub fn themes_dir() -> Option<PathBuf> {
    Some(config_dir()?.join("themes"))
}

/// Which theme to use; `System` follows the OS appearance.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    /// Whether the light theme applies, given the OS appearance.
    pub fn is_light(self, system_light: bool) -> bool {
        match self {
            Self::System => system_light,
            Self::Light => true,
            Self::Dark => false,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub repositories: Vec<PathBuf>,
    pub selected: Option<usize>,
    /// Side-by-side diff instead of unified (app-wide, like Desktop).
    pub diff_split: bool,
    /// Hide whitespace changes in diffs (`git diff -w`).
    pub diff_ignore_whitespace: bool,
    /// Wrap long diff lines instead of scrolling horizontally.
    pub diff_wrap: bool,
    pub theme: ThemeMode,
    /// Sidebar width in px; `None` = default (`gd_ui::theme::SIDEBAR_WIDTH`).
    pub sidebar_width: Option<f32>,
    /// Color theme family id (`gd_ui::themes::ColorThemes`).
    pub color_theme: String,
}

impl Settings {
    pub fn load() -> Self {
        let Some(path) = config_dir().map(|d| d.join("settings.toml")) else { return Self::default() };
        let Ok(text) = std::fs::read_to_string(&path) else { return Self::default() };
        toml::from_str(&text).unwrap_or_else(|e| {
            log::error!("cannot parse {}: {e}", path.display());
            Self::default()
        })
    }

    /// Id of the active color theme (`default` when unset or unknown).
    pub fn color_theme_id<'a>(&self, themes: &'a ColorThemes) -> &'a str {
        &themes.get(&self.color_theme).id
    }

    pub fn save(&self) {
        let Some(dir) = config_dir() else { return };
        let result = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(dir.join("settings.toml"), toml::to_string(self).unwrap_or_default()));
        if let Err(e) = result {
            log::error!("cannot save settings: {e}");
        }
    }
}

#[cfg(test)]
#[path = "tests/model.test.rs"]
mod tests;
