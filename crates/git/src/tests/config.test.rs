use super::*;

#[test]
fn list_records() {
    let out = "core.bare\nfalse\0remote.origin.fetch\n+refs/heads/*:refs/remotes/origin/*\0core.flag\0alias.log\nlog\n--oneline\0";
    let entries = parse_list(out);
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[1], ConfigEntry { key: "remote.origin.fetch".into(), value: "+refs/heads/*:refs/remotes/origin/*".into() });
    assert_eq!(entries[2].value, "");
    // A value can hold newlines: only the first one separates key from value.
    assert_eq!(entries[3].value, "log\n--oneline");
    assert!(parse_list("").is_empty());
}

#[test]
fn keys() {
    for ok in ["core.editor", "remote.origin.url", "branch.feat/x.remote", "user.signing-key", "a.b.c.d"] {
        assert!(validate_key(ok).is_ok(), "{ok}");
    }
    for bad in ["", "core", "core.", ".name", "1a.b", "core.1b", "co re.name", "core.na me", "core.a_b", "core.x\ny"] {
        assert!(validate_key(bad).is_err(), "{bad:?}");
    }
}
