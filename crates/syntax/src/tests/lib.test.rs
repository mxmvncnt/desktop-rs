use super::*;

fn run(path: &str, source: &str) -> HighlightedDocument {
    highlight(HighlightRequest {
        source,
        path: Some(path),
    })
    .unwrap()
}

#[test]
fn every_grammar_loads_and_highlights() {
    for (path, src) in [
        ("a.rs", "fn main() { let x = 1; } // hi"),
        ("a.js", "const a = () => 'x';"),
        ("a.jsx", "const a = <div>hi</div>;"),
        ("a.ts", "let a: number = 1;"),
        ("a.tsx", "const a: number = 1; <b/>"),
        ("a.py", "def f(x):\n    return 'a'"),
        ("a.go", "package main\nfunc main() {}"),
        ("a.json", "{\"a\": 1}"),
        ("a.sh", "echo \"hi\" | cat"),
        ("Cargo.toml", "[a]\nb = 1"),
        ("a.md", "# Title\n\n`code` and *em*\n"),
        (
            "a.html",
            "<div class=\"a\"><script>let a = 1;</script><style>a { color: red }</style></div>",
        ),
        ("a.css", "a { color: red; }"),
        ("a.yml", "a: 1\nb: [x]"),
        ("a.c", "int main() { return 0; }"),
        ("a.cpp", "class A { public: int x; };"),
    ] {
        let doc = run(path, src);
        assert!(
            doc.lines.iter().any(|l| !l.is_empty()),
            "{path} produced no spans"
        );
    }
}

#[test]
fn unknown_extension_is_an_error() {
    let req = |p| HighlightRequest {
        source: "x",
        path: Some(p),
    };
    assert_eq!(
        highlight(req("a.unknownext")).unwrap_err(),
        HighlightError::UnknownLanguage
    );
}

#[test]
fn spans_are_line_relative() {
    let doc = run("a.rs", "let a = 1;\nlet b = \"é\";\n");
    for (line, text) in doc.lines.iter().zip(["let a = 1;", "let b = \"é\";"]) {
        for s in line {
            assert!(s.range.end <= text.len() && text.is_char_boundary(s.range.start));
        }
    }
}
