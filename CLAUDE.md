# CLAUDE.md

This repository is **desktop-rs**, a Rust + GPUI-CE rewrite of GitHub Desktop.
The original Electron app is only in git history (`development` branch) and is
the behavioral reference, e.g. `git show development:app/src/...`.

Start here:
1. `docs/AGENTS.md`: hard rules. **Never touch files outside this
   project directory.**
2. `docs/PROGRESS.md`: current state and next steps.
3. `docs/ARCHITECTURE.md`: code map, threading, GPUI-CE gotchas.
4. `docs/DESIGN.md`: zeron visual language.
5. `docs/CONVENTIONS.md`: file/folder layout and code style. Follow it.

How to write code here (details in `docs/CONVENTIONS.md`):
- Minimal, readable code. No helper functions, wrappers or abstractions unless they remove
  real duplication (2+ call sites) or clearly help readability. Delete what you replace.
- One store per feature, one template (`Render`/`RenderOnce`) per file, generic components
  with local overrides in `components/`, shared non-UI helpers in `utils/`.
- Prefer a well-known crate over hand-rolled code. Git reads use `git2`; writes and network use the `git` CLI.
- Unit tests live in a `tests/` folder next to the source as `<file>.test.rs`, wired with `#[path]`.
- Before finishing: `cargo build`, `cargo build --features snapshot` (no warnings),
  `cargo clippy --workspace --all-targets`, `cargo test --workspace`, and the snapshots.
