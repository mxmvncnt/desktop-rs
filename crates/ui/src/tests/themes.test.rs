use super::*;

fn default_json(label: &str) -> String {
    let mut text = std::fs::read_to_string("assets/themes/default.json").unwrap();
    text = text.replace("\"Default\"", &format!("{label:?}"));
    text
}

#[test]
fn embedded_themes_parse() {
    let themes = ColorThemes::builtin();
    let ids: Vec<_> = themes.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["default", "catppuccin", "nord", "solarized", "tokyo-night", "vscode"]);
}

#[test]
fn rejects_bad_hex() {
    assert_eq!(serde_json::from_str::<Hex>("\"#1e1e2e\"").unwrap(), Hex(0x1e1e2e));
    for bad in ["\"1e1e2e\"", "\"#fff\"", "\"#gggggg\"", "\"#+12345\"", "12"] {
        assert!(serde_json::from_str::<Hex>(bad).is_err(), "{bad}");
    }
}

#[test]
fn user_theme_overrides_builtin_and_bad_files_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.json"), default_json("Mine")).unwrap();
    std::fs::write(dir.path().join("b.json"), default_json("Mine").replace("default", "extra")).unwrap();
    std::fs::write(dir.path().join("broken.json"), "{").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
    let mut themes = ColorThemes::builtin();
    themes.load_user(dir.path());
    assert_eq!(themes.iter().count(), 7);
    assert_eq!(themes.get("default").label, "Mine");
    assert_eq!(themes.get("extra").label, "Mine");
}

#[test]
fn unknown_id_falls_back_to_default() {
    assert_eq!(ColorThemes::builtin().get("bogus").id, DEFAULT_THEME);
}

#[test]
fn accepts_schema_key() {
    let with = default_json("Default");
    assert!(serde_json::from_str::<ColorTheme>(&with).unwrap().schema.is_some());
    let without = with.lines().filter(|l| !l.contains("$schema")).collect::<Vec<_>>().join("\n");
    assert!(serde_json::from_str::<ColorTheme>(&without).unwrap().schema.is_none());
}

#[test]
fn checked_in_schema_is_current() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/theme.schema.json");
    let generated = serde_json::to_string_pretty(&schemars::schema_for!(ColorTheme)).unwrap() + "\n";
    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        std::fs::write(path, &generated).unwrap();
    }
    assert!(std::fs::read_to_string(path).unwrap() == generated, "docs/theme.schema.json is stale: run `UPDATE_SCHEMA=1 cargo test -p gd-ui checked_in_schema`");
}

fn default_palette() -> Palette {
    ColorThemes::builtin().get(DEFAULT_THEME).dark
}

#[test]
fn syntax_colors_ignore_the_theme() {
    let themes = ColorThemes::builtin();
    for light in [false, true] {
        let syntax = |t: &ColorTheme| Theme::from_palette(if light { &t.light } else { &t.dark }, light).syntax;
        let reference = syntax(themes.get(DEFAULT_THEME));
        for other in themes.iter() {
            let s = syntax(other);
            assert_eq!(SyntaxPalette { text: reference.text, ..s }, reference, "{}", other.id);
        }
    }
}

#[test]
fn removed_syntax_keys_are_still_accepted() {
    let old = default_json("Old").replace("\"on_accent\": \"#fafafa\"", "\"on_accent\": \"#fafafa\", \"comment\": \"#808080\", \"keyword\": \"#8f96ed\"");
    assert!(old.contains("\"keyword\""));
    assert!(serde_json::from_str::<ColorTheme>(&old).is_ok());
}

#[test]
fn diff_overrides_are_applied() {
    let p = Palette {
        diff_add_bg: Some(Hex(0xe6ffed)),
        diff_add_gutter_bg: Some(Hex(0xcdffd8)),
        diff_hunk_gutter_bg: Some(Hex(0xdbedff)),
        diff_empty_bg: Some(Hex(0xfafbfc)),
        diff_gutter: Some(Hex(0x959da5)),
        ..default_palette()
    };
    let t = Theme::from_palette(&p, true);
    assert_eq!(t.diff_add_bg, rgb_to_hsla(rgb(0xe6ffed)));
    assert_eq!(t.diff_add_gutter_bg, rgb_to_hsla(rgb(0xcdffd8)));
    assert_eq!(t.diff_hunk_gutter_bg, rgb_to_hsla(rgb(0xdbedff)));
    assert_eq!(t.diff_empty_bg, rgb_to_hsla(rgb(0xfafbfc)));
    assert_eq!(t.diff_gutter, rgb_to_hsla(rgb(0x959da5)));
}

#[test]
fn absent_diff_overrides_keep_derived_colors() {
    for light in [false, true] {
        let p = default_palette();
        let t = Theme::from_palette(&p, light);
        assert_eq!(t.diff_add_gutter_bg, t.diff_add_bg);
        assert_eq!(t.diff_del_gutter_bg, t.diff_del_bg);
        assert_eq!(t.diff_gutter_bg, t.diff_empty_bg);
        assert!((t.diff_gutter_bg.alpha - t.ink(0.025).alpha).abs() < 1e-6);
        assert_eq!(t.diff_gutter, rgb_to_hsla(rgb(p.faint.0)));
    }
}

#[test]
fn partial_diff_override_keeps_other_gutters_following_line_bg() {
    let p = Palette { diff_del_bg: Some(Hex(0xffeef0)), ..default_palette() };
    let t = Theme::from_palette(&p, true);
    assert_eq!(t.diff_del_gutter_bg, rgb_to_hsla(rgb(0xffeef0)));
}
