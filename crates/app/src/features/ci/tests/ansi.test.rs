use super::*;

#[test]
fn base_colors_depend_on_the_theme() {
    assert_eq!(hex(Color::Indexed(1), false), 0xcd3131);
    assert_eq!(hex(Color::Indexed(0), true), 0x000000);
    assert_ne!(hex(Color::Indexed(0), false), hex(Color::Indexed(0), true));
}

#[test]
fn cube_and_gray_ramp() {
    assert_eq!(hex(Color::Indexed(16), false), 0x000000);
    assert_eq!(hex(Color::Indexed(231), false), 0xffffff);
    assert_eq!(hex(Color::Indexed(196), false), 0xff0000);
    assert_eq!(hex(Color::Indexed(232), false), 0x080808);
    assert_eq!(hex(Color::Indexed(255), false), 0xeeeeee);
}

#[test]
fn rgb_is_passed_through() {
    assert_eq!(hex(Color::Rgb(1, 2, 3), true), 0x010203);
}
