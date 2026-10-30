//! Design tokens, ported from zeron's `crates/ui/src/theme.rs`.
//!
//! Zeron's dark palette is a near-black elevation ladder with tiny lightness
//! steps (#060606 → #0d0d0d → #0e → #10 → #16) plus translucent white "ink"
//! for hover/active washes and hairlines. Keep new colors on this ladder —
//! don't invent opaque greys, use `theme.ink()` instead. Themes are built from
//! palettes in `themes.rs`.

use gd_syntax::HighlightKind;
use gpui::{App, Global, Hsla, SharedString, hsla, rgb, rgb_to_hsla};

#[derive(Debug, Clone)]
pub struct Theme {
    /// Light appearance: `ink` washes are black instead of white.
    pub light: bool,
    // ---- neutral surfaces ----
    /// Main content plane (diff view). Deepest.
    pub bg: Hsla,
    /// Shell / sidebar / toolbar. One step up from `bg`.
    pub surface: Hsla,
    /// Opaque raised pills/chips.
    pub surface_raised: Hsla,
    pub surface_raised_hover: Hsla,
    /// Inline cards on the main panel.
    pub surface_card: Hsla,
    /// Popovers, menus — the highest plane.
    pub surface_overlay: Hsla,
    pub element_hover: Hsla,
    pub element_active: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,

    // ---- text ----
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,

    // ---- high-contrast plate (primary button) ----
    pub solid: Hsla,
    pub solid_hover: Hsla,
    pub on_solid: Hsla,

    // ---- accents ----
    pub accent: Hsla,
    pub accent_strong: Hsla,
    pub accent_wash: Hsla,
    pub on_accent: Hsla,
    pub danger: Hsla,
    pub danger_strong: Hsla,
    pub warning: Hsla,
    pub success: Hsla,

    // ---- inputs ----
    pub input_bg: Hsla,
    pub selection: Hsla,
    pub caret: Hsla,

    // ---- diff ----
    pub diff_add: Hsla,
    pub diff_del: Hsla,
    pub diff_add_bg: Hsla,
    pub diff_del_bg: Hsla,
    /// Word-level emphasis inside changed lines (stronger wash).
    pub diff_add_emph: Hsla,
    pub diff_del_emph: Hsla,
    pub diff_hunk_bg: Hsla,
    /// Line-number cell backgrounds per line kind, and the hunk-header gutter (expand buttons, handle column).
    pub diff_add_gutter_bg: Hsla,
    pub diff_del_gutter_bg: Hsla,
    pub diff_hunk_gutter_bg: Hsla,
    pub diff_gutter_bg: Hsla,
    /// Split view placeholder opposite an added/deleted line.
    pub diff_empty_bg: Hsla,
    pub diff_gutter: Hsla,

    // ---- syntax ----
    pub syntax: SyntaxPalette,

    // ---- fonts ----
    pub font_sans: SharedString,
    pub font_mono: SharedString,
}

impl Global for Theme {}

// Sizes (px), from zeron's Theme consts.
pub const TOOLBAR_HEIGHT: f32 = 52.0;
pub const PANEL_RADIUS: f32 = 10.0;
pub const CONTROL_RADIUS: f32 = 6.0;
pub const SIDEBAR_WIDTH: f32 = 300.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 220.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 520.0;
pub const ROW_HEIGHT: f32 = 30.0;
pub const DIFF_ROW_HEIGHT: f32 = 20.0;

impl Theme {
    pub fn of(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Translucent fill for hover/active plates: white on dark, black on light.
    pub fn ink(&self, alpha: f32) -> Hsla {
        if self.light { hsla(0.0, 0.0, 0.0, alpha * 0.8) } else { hsla(0.0, 0.0, 1.0, alpha) }
    }
}

/// Paint-only syntax colors: seven base colors that [`SyntaxPalette::color`]
/// maps onto every highlight kind (zeron's scheme: indigo keywords/functions,
/// emerald strings, pink specials, amber numbers/types).
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxPalette {
    pub text: Hsla,
    pub comment: Hsla,
    pub indigo: Hsla,
    pub pink: Hsla,
    pub emerald: Hsla,
    pub amber: Hsla,
    pub red: Hsla,
}

impl SyntaxPalette {
    /// Fixed syntax colors (not themeable): one set for dark and one for light, only plain `text` follows the theme.
    pub fn new(light: bool, text: Hsla) -> Self {
        let c = |dark: u32, light_: u32| rgb_to_hsla(rgb(if light { light_ } else { dark }));
        Self {
            text,
            comment: c(0x808080, 0x7a7a7a),
            indigo: c(0x8f96ed, 0x6454dc),
            pink: c(0xe679b4, 0xc62075),
            emerald: c(0x1eb787, 0x11694e),
            amber: c(0xdba924, 0xa0511a),
            red: c(0xff6467, 0xe7000b),
        }
    }

    /// Color for a highlight kind. Called once per span while building runs (never per frame).
    pub fn color(&self, kind: HighlightKind) -> Hsla {
        use HighlightKind::*;
        match kind {
            Comment => self.comment,
            Keyword | Function | MarkupHeading | MarkupStrong => self.indigo,
            StringSpecial | Escape | FunctionBuiltin | Macro | VariableSpecial | Tag | MarkupLink | MarkupEmphasis => {
                self.pink
            }
            String | TypeBuiltin | Constant | MarkupRaw => self.emerald,
            Number | Boolean | Type | Constructor | Property | Attribute | Label | MarkupReference => self.amber,
            Variable | Parameter | Operator | Punctuation | Embedded => self.text,
            Invalid => self.red,
        }
    }
}

/// Modal backdrop.
pub fn scrim(alpha: f32) -> Hsla {
    hsla(0.0, 0.0, 0.0, alpha)
}
