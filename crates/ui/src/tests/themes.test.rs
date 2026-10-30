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
