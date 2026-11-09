# desktop-rs, explained for humans

This folder explains the app to a person who did not write it. It assumes you know
some programming (you know Vue, for example) but not Rust, not GPUI, and not this
code base.

The other files in `docs/` (`AGENTS.md`, `ARCHITECTURE.md`, `CONVENTIONS.md`,
`PROGRESS.md`, `CLEANUP_REPORT.md`) were written by and for AI sessions. They are
very detailed and sometimes out of date. Read this folder first; go there when you
need the fine print on one topic.

## Reading order

| # | File | What you get out of it |
|---|---|---|
| 1 | [01-what-is-this.md](01-what-is-this.md) | What the app is, the 5 pieces it is made of, one picture of how they talk |
| 2 | [02-reading-rust-and-gpui.md](02-reading-rust-and-gpui.md) | Just enough Rust and GPUI to read the code, with Vue comparisons |
| 3 | [03-how-it-works.md](03-how-it-works.md) | What happens when you start the app, click a file, commit, press Esc |
| 4 | [04-code-map.md](04-code-map.md) | Every folder, and which file to open for which part of the screen |
| 5 | [05-recipes.md](05-recipes.md) | Step-by-step: add a shortcut, a menu item, a dialog, a setting, a git command... |
| 6 | [06-build-test-debug.md](06-build-test-debug.md) | Commands to build, test, take screenshots, debug and release |
| 7 | [07-glossary.md](07-glossary.md) | Words used in the code (git words and app words) |
| 8 | [08-find-and-debug.md](08-find-and-debug.md) | Screen → file: how to find the code behind what you see, and how to debug it (logs, breakpoints, snapshot views, headless key tests) |
| - | [cleanup-todo.md](cleanup-todo.md) | The list of things to clean up, with simple steps, in a sensible order |

## The five-second version

- It's a Rust rewrite of GitHub Desktop. `cargo run` starts it.
- Everything git-related is in `crates/git` and works by running the real `git`
  command and reading its output.
- Everything on screen is in `crates/app/src/features/<feature>/`, one folder per
  feature (stashes, history, merge, CI page, ...).
- Two big objects hold almost all the state:
  - `Workspace` (`crates/app/src/workspace/`) is the window: which menus and dialogs are
    open, text inputs, scroll positions, settings.
  - `RepoModel` (`crates/app/src/repo/`) is the open repository: files changed, commits,
    branches, the selected diff.
- Git commands never run on the UI thread. They run in the background, and the result
  is applied afterwards. If you add one, copy an existing one (see recipes).
- Before you commit: `cargo build`, `cargo clippy --workspace --all-targets`,
  `cargo test --workspace`. See [06-build-test-debug.md](06-build-test-debug.md).

## If you only have 30 minutes

1. Read [01-what-is-this.md](01-what-is-this.md) and the "Life of a click" part of
   [03-how-it-works.md](03-how-it-works.md).
2. Open `crates/app/src/features/stashes/`. It is the cleanest feature and the model
   the others copy. Read `mod.rs`, then `state.rs`, `ops.rs`, `handlers.rs`, `stash_row.rs`.
3. Run the app on a test repository and match what you see to those files.
