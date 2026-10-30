//! Data-driven color themes. Each [`ColorTheme`] is a family with a dark and a
//! light [`Palette`] (a handful of hex values), loaded from JSON: the built-ins
//! are embedded (`assets/themes/<id>.json`), user themes are every `*.json` in
//! the config dir's `themes/` folder (same `id` replaces a built-in).
//! [`Theme::from_palette`] expands a palette into the full token set, so it is
//! the only `Theme` constructor. To add a theme, drop a JSON file; the settings
//! page and persistence pick it up by `id`.
//!
//! Mode rule: every family has both variants, so System/Light/Dark simply picks
//! the variant. The default theme is the first entry.

use std::path::Path;

use gpui::{App, ColorExt as _, Global, Hsla, Rgba, hsla, rgb, rgb_to_hsla};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::assets::Assets;
use crate::theme::{SyntaxPalette, Theme};

/// A `#rrggbb` color from JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
#[schemars(with = "String", extend("pattern" = "^#[0-9a-fA-F]{6}$"))]
pub struct Hex(pub u32);

impl TryFrom<String> for Hex {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        let digits = s.strip_prefix('#').filter(|d| d.len() == 6 && d.bytes().all(|b| b.is_ascii_hexdigit()));
        digits.and_then(|d| u32::from_str_radix(d, 16).ok()).map(Self).ok_or(format!("invalid color {s:?}, expected \"#rrggbb\""))
    }
}

/// Source colors of one theme variant.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
pub struct Palette {
    /// Diff plane (the deepest surface in dark variants).
    pub bg: Hex,
    /// Toolbar / sidebar.
    pub surface: Hex,
    /// Cards and panels.
    pub card: Hex,
    /// Popovers and menus.
    pub overlay: Hex,
    /// Opaque chips and pills.
    pub raised: Hex,
    /// Primary text.
    pub text: Hex,
    /// Secondary text.
    pub muted: Hex,
    /// Tertiary text, gutters, disabled.
    pub faint: Hex,
    /// Accent color: links, selection, focus.
    pub accent: Hex,
    /// Stronger accent for pressed or emphasized states.
    pub accent_strong: Hex,
    /// Errors and deleted lines.
    pub danger: Hex,
    /// Warnings and modified files.
    pub warning: Hex,
    /// Success and added lines.
    pub success: Hex,
    /// Text on accent fills.
    pub on_accent: Hex,
    /// Code comments.
    pub comment: Hex,
    /// Syntax base colors: keywords/functions, strings, numbers/types, specials.
    pub keyword: Hex,
    pub string: Hex,
    pub number: Hex,
    pub special: Hex,
}

/// A color theme family: one palette per appearance.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ColorTheme {
    /// Editor hint: URL or path of the JSON Schema. Ignored by the app.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    /// Unique id, persisted in settings. The same id as a built-in replaces it.
    pub id: String,
    /// Name shown in Settings > Appearance.
    pub label: String,
    /// Palette used in dark mode.
    pub dark: Palette,
    /// Palette used in light mode.
    pub light: Palette,
}

pub const DEFAULT_THEME: &str = "default";

/// Every available color theme (global): built-ins first, then user themes.
pub struct ColorThemes(Vec<ColorTheme>);

impl Global for ColorThemes {}

impl ColorThemes {
    pub fn of(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// The embedded themes, default first then by id. A broken one is a programmer error.
    pub fn builtin() -> Self {
        let mut themes: Vec<ColorTheme> = Assets::iter()
            .filter(|p| p.starts_with("themes/"))
            .map(|p| {
                let data = Assets::get(&p).expect("listed asset exists").data;
                serde_json::from_slice(&data).unwrap_or_else(|e| panic!("embedded theme {p}: {e}"))
            })
            .collect();
        themes.sort_by(|a, b| (a.id != DEFAULT_THEME, &a.id).cmp(&(b.id != DEFAULT_THEME, &b.id)));
        Self(themes)
    }

    /// Add every `*.json` in `dir` (by file name); a file that fails to parse is logged and skipped.
    pub fn load_user(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
        paths.sort();
        for path in paths {
            let parsed = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()));
            match parsed {
                Ok(theme) => self.add(theme),
                Err(e) => log::error!("cannot load theme {}: {e}", path.display()),
            }
        }
    }

    fn add(&mut self, theme: ColorTheme) {
        match self.0.iter_mut().find(|t| t.id == theme.id) {
            Some(existing) => *existing = theme,
            None => self.0.push(theme),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ColorTheme> {
        self.0.iter()
    }

    /// The theme with this id; unknown ids fall back to default.
    pub fn get(&self, id: &str) -> &ColorTheme {
        self.0.iter().find(|t| t.id == id).unwrap_or(&self.0[0])
    }
}

impl Theme {
    /// Theme for a family id (unknown ids fall back to default) and resolved appearance.
    pub fn for_family(id: &str, light: bool, cx: &App) -> Self {
        let family = ColorThemes::of(cx).get(id);
        Self::from_palette(if light { &family.light } else { &family.dark }, light)
    }

    fn from_palette(p: &Palette, light: bool) -> Self {
        let c = |v: Hex| rgb_to_hsla(rgb(v.0));
        let (accent, danger, success) = (c(p.accent), c(p.danger), c(p.success));
        // Translucent washes: white on dark, black on light.
        let (wash, hover, active, border, border_strong) = if light {
            (0.0, 0.05, 0.09, 0.10, 0.18)
        } else {
            (1.0, 0.07, 0.11, 0.08, 0.14)
        };
        let (add_wash, emph) = if light { (0.12, 0.28) } else { (0.10, 0.30) };
        Self {
            light,
            bg: c(p.bg),
            surface: c(p.surface),
            surface_raised: c(p.raised),
            surface_raised_hover: mix(p.raised.0, p.text.0, 0.10),
            surface_card: c(p.card),
            surface_overlay: c(p.overlay),
            element_hover: hsla(0.0, 0.0, if light { 0.0 } else { 0.92 }, hover),
            element_active: hsla(0.0, 0.0, if light { 0.0 } else { 0.92 }, active),
            border: hsla(0.0, 0.0, wash, border),
            border_strong: hsla(0.0, 0.0, wash, border_strong),
            text: c(p.text),
            text_muted: c(p.muted),
            text_faint: c(p.faint),
            solid: c(p.text),
            solid_hover: hsla(0.0, 0.0, wash, 1.0),
            on_solid: c(p.bg),
            accent,
            accent_strong: c(p.accent_strong),
            accent_wash: accent.opacity(0.14),
            on_accent: c(p.on_accent),
            danger,
            danger_strong: mix(p.danger.0, 0x000000, 0.24),
            warning: c(p.warning),
            success,
            input_bg: hsla(0.0, 0.0, wash, 0.03),
            selection: accent.opacity(0.30),
            caret: accent,
            diff_add: success,
            diff_del: danger,
            diff_add_bg: success.opacity(add_wash),
            diff_del_bg: danger.opacity(add_wash),
            diff_add_emph: success.opacity(emph),
            diff_del_emph: danger.opacity(emph),
            diff_hunk_bg: accent.opacity(0.08),
            diff_gutter: c(p.faint),
            syntax: SyntaxPalette {
                text: c(p.text),
                comment: c(p.comment),
                indigo: c(p.keyword),
                pink: c(p.special),
                emerald: c(p.string),
                amber: c(p.number),
                red: danger,
            },
            font_sans: "Geist".into(),
            font_mono: "Geist Mono".into(),
        }
    }
}

/// `a` blended toward `b` by `t` (sRGB space, fine for small steps).
fn mix(a: u32, b: u32, t: f32) -> Hsla {
    let ch = |v: u32, shift: u32| ((v >> shift) & 0xff) as f32 / 255.0;
    let l = |shift: u32| ch(a, shift) + (ch(b, shift) - ch(a, shift)) * t;
    rgb_to_hsla(Rgba::new(l(16), l(8), l(0), 1.0))
}

#[cfg(test)]
#[path = "tests/themes.test.rs"]
mod tests;
