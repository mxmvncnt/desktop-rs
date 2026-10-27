use super::*;

#[test]
fn parses_trailers() {
    let m = "subject\n\ntext\n\nCo-authored-by: Alice A <a@x.io>\nco-Authored-By: Bob <b@x.io>\nco-authored-by: Alice A <a@x.io>";
    let c = parse_co_authors(m);
    assert_eq!(c.len(), 2);
    assert_eq!((c[0].name.as_str(), c[0].email.as_str()), ("Alice A", "a@x.io"));
    assert_eq!(c[1].name, "Bob");
}
