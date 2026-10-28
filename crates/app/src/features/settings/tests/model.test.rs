use super::*;

#[test]
fn round_trips_and_tolerates_missing_keys() {
    let s: Settings = toml::from_str("diff_split = true\ntheme = \"dark\"\nrepositories = [\"/a\", \"/b c\"]\nselected = 1").unwrap();
    assert!(s.diff_split && !s.diff_wrap);
    assert_eq!((s.theme, s.selected), (ThemeMode::Dark, Some(1)));
    let t: Settings = toml::from_str(&toml::to_string(&s).unwrap()).unwrap();
    assert_eq!((t.repositories, t.sidebar_width), (s.repositories, None));
    let u: Settings = toml::from_str("sidebar_width = 345.0\ncolor_theme = \"nord\"").unwrap();
    assert_eq!((u.sidebar_width, u.color_theme_id()), (Some(345.0), "nord"));
    assert_eq!(Settings { color_theme: "bogus".into(), ..Settings::default() }.color_theme_id(), "zeron");
}
