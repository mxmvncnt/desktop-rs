//! Data-driven color themes. Each [`ColorTheme`] is a family with a dark and a
//! light [`Palette`] (a handful of hex values); [`Theme::from_palette`] expands a
//! palette into the full token set, so it is the only `Theme` constructor. To
//! add a theme, append one entry to [`COLOR_THEMES`]; the settings page and
//! persistence pick it up by `id`.
//!
//! Mode rule: every family has both variants, so System/Light/Dark simply picks
//! the variant. Zeron (the default) is the first entry.

use gpui::{ColorExt as _, Hsla, Rgba, hsla, rgb, rgb_to_hsla};

use crate::theme::{SyntaxPalette, Theme};

/// Source colors of one theme variant, as `0xRRGGBB`.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Diff plane (the deepest surface in dark variants).
    pub bg: u32,
    /// Toolbar / sidebar.
    pub surface: u32,
    pub card: u32,
    /// Popovers and menus.
    pub overlay: u32,
    /// Opaque chips and pills.
    pub raised: u32,
    pub text: u32,
    pub muted: u32,
    pub faint: u32,
    pub accent: u32,
    pub accent_strong: u32,
    pub danger: u32,
    pub warning: u32,
    pub success: u32,
    /// Text on accent fills.
    pub on_accent: u32,
    pub comment: u32,
    /// Syntax base colors: keywords/functions, strings, numbers/types, specials.
    pub keyword: u32,
    pub string: u32,
    pub number: u32,
    pub special: u32,
}

pub struct ColorTheme {
    pub id: &'static str,
    pub label: &'static str,
    pub dark: Palette,
    pub light: Palette,
}

pub const DEFAULT_THEME: &str = "zeron";

pub fn find(id: &str) -> Option<&'static ColorTheme> {
    COLOR_THEMES.iter().find(|t| t.id == id)
}

pub static COLOR_THEMES: &[ColorTheme] = &[
    ColorTheme { id: DEFAULT_THEME, label: "Zeron", dark: ZERON_DARK, light: ZERON_LIGHT },
    ColorTheme { id: "nord", label: "Nord", dark: NORD_DARK, light: NORD_LIGHT },
    ColorTheme { id: "tokyo-night", label: "Tokyo Night", dark: TOKYO_NIGHT, light: TOKYO_DAY },
    ColorTheme { id: "vscode", label: "VS Code", dark: VSCODE_DARK, light: VSCODE_LIGHT },
    ColorTheme { id: "catppuccin", label: "Catppuccin", dark: MOCHA, light: LATTE },
    ColorTheme { id: "solarized", label: "Solarized", dark: SOLARIZED_DARK, light: SOLARIZED_LIGHT },
];

impl Theme {
    /// Theme for a family id (unknown ids fall back to zeron) and resolved appearance.
    pub fn for_family(id: &str, light: bool) -> Self {
        let family = find(id).unwrap_or(&COLOR_THEMES[0]);
        Self::from_palette(if light { &family.light } else { &family.dark }, light)
    }

    fn from_palette(p: &Palette, light: bool) -> Self {
        let c = |v: u32| rgb_to_hsla(rgb(v));
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
            surface_raised_hover: mix(p.raised, p.text, 0.10),
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
            solid_hover: hsla(0.0, 0.0, 1.0 - wash, 1.0),
            on_solid: c(p.bg),
            accent,
            accent_strong: c(p.accent_strong),
            accent_wash: accent.opacity(0.14),
            on_accent: c(p.on_accent),
            danger,
            danger_strong: mix(p.danger, 0x000000, 0.24),
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

const ZERON_DARK: Palette = Palette {
    bg: 0x060606, surface: 0x0d0d0d, card: 0x0e0e0e, overlay: 0x161616, raised: 0x1e1e1e,
    text: 0xe5e5e5, muted: 0xa1a1a1, faint: 0x737373,
    accent: 0x51a2ff, accent_strong: 0x155dfc, danger: 0xff6467, warning: 0xffb900, success: 0x00d492,
    on_accent: 0xfafafa, comment: 0x808080, keyword: 0x8f96ed, string: 0x1eb787, number: 0xdba924, special: 0xe679b4,
};

const ZERON_LIGHT: Palette = Palette {
    bg: 0xffffff, surface: 0xf6f6f6, card: 0xfafafa, overlay: 0xffffff, raised: 0xebebeb,
    text: 0x171717, muted: 0x484848, faint: 0x696969,
    accent: 0x155dfc, accent_strong: 0x1447e6, danger: 0xe7000b, warning: 0xe17100, success: 0x009966,
    on_accent: 0xfafafa, comment: 0x7a7a7a, keyword: 0x6454dc, string: 0x11694e, number: 0xa0511a, special: 0xc62075,
};

const NORD_DARK: Palette = Palette {
    bg: 0x2e3440, surface: 0x272c36, card: 0x2b303b, overlay: 0x3b4252, raised: 0x434c5e,
    text: 0xd8dee9, muted: 0xaab2c0, faint: 0x7b88a1,
    accent: 0x88c0d0, accent_strong: 0x5e81ac, danger: 0xd0707a, warning: 0xebcb8b, success: 0xa3be8c,
    on_accent: 0x2e3440, comment: 0x616e88, keyword: 0x81a1c1, string: 0xa3be8c, number: 0xb48ead, special: 0xebcb8b,
};

const NORD_LIGHT: Palette = Palette {
    bg: 0xeceff4, surface: 0xe5e9f0, card: 0xe9edf3, overlay: 0xf6f8fb, raised: 0xd8dee9,
    text: 0x2e3440, muted: 0x4c566a, faint: 0x6b7690,
    accent: 0x5e81ac, accent_strong: 0x4c6f9a, danger: 0xbf616a, warning: 0xb5832a, success: 0x5f8a4a,
    on_accent: 0xffffff, comment: 0x8a93a6, keyword: 0x5e81ac, string: 0x5f8a4a, number: 0x9b6a95, special: 0xb5832a,
};

const TOKYO_NIGHT: Palette = Palette {
    bg: 0x1a1b26, surface: 0x16161e, card: 0x1c1d29, overlay: 0x24283b, raised: 0x292e42,
    text: 0xc0caf5, muted: 0x9aa5ce, faint: 0x565f89,
    accent: 0x7aa2f7, accent_strong: 0x3d59a1, danger: 0xf7768e, warning: 0xe0af68, success: 0x9ece6a,
    on_accent: 0x1a1b26, comment: 0x565f89, keyword: 0xbb9af7, string: 0x9ece6a, number: 0xff9e64, special: 0xe0af68,
};

const TOKYO_DAY: Palette = Palette {
    bg: 0xe1e2e7, surface: 0xd5d6db, card: 0xdcdde3, overlay: 0xf0f0f5, raised: 0xc8cad3,
    text: 0x343b58, muted: 0x4c505e, faint: 0x6172b0,
    accent: 0x2e7de9, accent_strong: 0x3760bf, danger: 0xf52a65, warning: 0x8c6c3e, success: 0x587539,
    on_accent: 0xffffff, comment: 0x848cb5, keyword: 0x9854f1, string: 0x587539, number: 0xb15c00, special: 0x8c6c3e,
};

const VSCODE_DARK: Palette = Palette {
    bg: 0x1e1e1e, surface: 0x181818, card: 0x202020, overlay: 0x252526, raised: 0x37373d,
    text: 0xd4d4d4, muted: 0x9d9d9d, faint: 0x6e7681,
    accent: 0x3794ff, accent_strong: 0x0e639c, danger: 0xf48771, warning: 0xcca700, success: 0x89d185,
    on_accent: 0x1e1e1e, comment: 0x6a9955, keyword: 0x569cd6, string: 0xce9178, number: 0xb5cea8, special: 0xd7ba7d,
};

const VSCODE_LIGHT: Palette = Palette {
    bg: 0xffffff, surface: 0xf3f3f3, card: 0xf8f8f8, overlay: 0xffffff, raised: 0xe8e8e8,
    text: 0x3b3b3b, muted: 0x616161, faint: 0x8a8a8a,
    accent: 0x0066bf, accent_strong: 0x005fb8, danger: 0xcd3131, warning: 0xbf8803, success: 0x388a34,
    on_accent: 0xffffff, comment: 0x008000, keyword: 0x0000ff, string: 0xa31515, number: 0x098658, special: 0xe50000,
};

const MOCHA: Palette = Palette {
    bg: 0x1e1e2e, surface: 0x181825, card: 0x1a1a29, overlay: 0x313244, raised: 0x313244,
    text: 0xcdd6f4, muted: 0xa6adc8, faint: 0x7f849c,
    accent: 0x89b4fa, accent_strong: 0x5b8bdc, danger: 0xf38ba8, warning: 0xf9e2af, success: 0xa6e3a1,
    on_accent: 0x1e1e2e, comment: 0x6c7086, keyword: 0xcba6f7, string: 0xa6e3a1, number: 0xfab387, special: 0xf2cdcd,
};

const LATTE: Palette = Palette {
    bg: 0xeff1f5, surface: 0xe6e9ef, card: 0xebedf2, overlay: 0xfcfcfd, raised: 0xccd0da,
    text: 0x4c4f69, muted: 0x5c5f77, faint: 0x8c8fa1,
    accent: 0x1e66f5, accent_strong: 0x1a56d6, danger: 0xd20f39, warning: 0xdf8e1d, success: 0x40a02b,
    on_accent: 0xffffff, comment: 0x9ca0b0, keyword: 0x8839ef, string: 0x40a02b, number: 0xfe640b, special: 0xdd7878,
};

const SOLARIZED_DARK: Palette = Palette {
    bg: 0x002b36, surface: 0x00212b, card: 0x00252f, overlay: 0x073642, raised: 0x0b3f4c,
    text: 0x93a1a1, muted: 0x839496, faint: 0x657b83,
    accent: 0x268bd2, accent_strong: 0x1e6fa8, danger: 0xdc322f, warning: 0xb58900, success: 0x859900,
    on_accent: 0x002b36, comment: 0x586e75, keyword: 0x859900, string: 0x2aa198, number: 0xd33682, special: 0xd33682,
};

const SOLARIZED_LIGHT: Palette = Palette {
    bg: 0xfdf6e3, surface: 0xeee8d5, card: 0xf5eed9, overlay: 0xfffaf0, raised: 0xe4ddc8,
    text: 0x586e75, muted: 0x657b83, faint: 0x93a1a1,
    accent: 0x268bd2, accent_strong: 0x1e6fa8, danger: 0xdc322f, warning: 0xb58900, success: 0x859900,
    on_accent: 0xffffff, comment: 0x93a1a1, keyword: 0x859900, string: 0x2aa198, number: 0xd33682, special: 0xd33682,
};
