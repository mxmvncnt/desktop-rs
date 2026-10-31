//! ANSI colors of job logs: the 16 terminal colors (one set per theme brightness), the 6x6x6 cube, the gray ramp and 24-bit.

use gd_forge::Color;
use gpui::Hsla;

const DARK: [u32; 16] = [
    0x3b3b3b, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5, 0x666666, 0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea,
    0xd670d6, 0x29b8db, 0xffffff,
];
const LIGHT: [u32; 16] = [
    0x000000, 0xcd3131, 0x00a000, 0x949800, 0x0451a5, 0xbc05bc, 0x0598bc, 0x555555, 0x666666, 0xcd3131, 0x14a014, 0xb5ba00, 0x0451a5,
    0xbc05bc, 0x0598bc, 0xa5a5a5,
];

/// `0xrrggbb`.
fn hex(color: Color, light: bool) -> u32 {
    match color {
        Color::Rgb(r, g, b) => u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b),
        Color::Indexed(i @ 0..=15) => {
            let palette = if light { LIGHT } else { DARK };
            palette[usize::from(i)]
        }
        Color::Indexed(i @ 16..=231) => {
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * u32::from(v) };
            let n = i - 16;
            level(n / 36) << 16 | level(n / 6 % 6) << 8 | level(n % 6)
        }
        Color::Indexed(i) => {
            let gray = 8 + 10 * u32::from(i - 232);
            gray << 16 | gray << 8 | gray
        }
    }
}

pub fn ansi_color(color: Color, light: bool) -> Hsla {
    gpui::rgb_to_hsla(gpui::rgb(hex(color, light)))
}

#[cfg(test)]
#[path = "tests/ansi.test.rs"]
mod tests;
