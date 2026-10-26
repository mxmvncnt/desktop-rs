# Rules for AI sessions and subagents

## Hard rules

1. **Never read, create, or modify files outside of this directory**
   (exceptions: cargo's own cache in `~/.cargo`, and `/tmp/desktop-rs/`).
   - `/tmp/desktop-rs/` may be read and written freely. Scratch repos,
     screenshots, and logs go there (`mkdir -p` it). Nothing else under `/tmp`
     is allowed.
   - When running the app, set `GD_CONFIG_DIR=/tmp/desktop-rs/config`. Otherwise it
     writes to the real `~/.config/desktop-rs`.
   - Don't `git clone` or `curl` into `~` or anywhere in `/tmp` except `/tmp/desktop-rs/`. Reference repos live in `.ref/`.
   - Don't touch the user's global git config or other repos.
2. Only touch files inside this repository. The original Electron app (removed
   from this tree) lives in git history, e.g. `git show development:app/src/...`;
   read it for behavior, don't try to restore it.
3. Work on your own branch and commit your own work (see Workflow). Never
   push or force-push, and never rewrite `main` history.
4. Performance is a primary goal. Follow the rules in ARCHITECTURE.md
   §Performance: no git on the UI thread, virtualize lists, no idle repeating animations.

## Workflow for parallel subagents

- Integration branch: `main` (or the current cleanup branch the coordinator names).
- Each subagent gets its own git worktree inside the project:
  ```sh
  cd <repo-root>
  git worktree add .worktrees/<name> -b rust/<name> main
  # Don't copy target/ from another checkout: cargo fingerprints include the
  # absolute path, so it rebuilds everything anyway (~4 min cold, ~7.5 GB).
  cd .worktrees/<name>
  ```
- Commit in small, logical commits with clear messages on `rust/<name>`.
- Before your final commit: `cargo build` and `cargo build --features snapshot`
  must be warning-free, and you must check your UI with a `--snapshot` PNG.
- The coordinator (main session) merges `rust/<name>` into `main` and
  removes the worktree.

## Verifying UI without a display

```sh
cargo run --features snapshot -- --snapshot /tmp/desktop-rs/x.png <repo> <view>
```

Then look at the PNG. Make test repos under `/tmp/desktop-rs/` (see PROGRESS.md for a
script). The snapshot can't click, but it can hover (`GD_SNAPSHOT_MOUSE=x,y`) and
drag with the left button (`GD_SNAPSHOT_DRAG=x1,y1,x2,y2`, logical window px). For
other interaction states, add a view name to `crates/app/src/snapshot.rs` that sets
up the state directly.

## Documentation duty

Every change that adds a feature, a gotcha, or a decision updates:

- `docs/PROGRESS.md`: tick the feature and add notes.
- `docs/ARCHITECTURE.md`: for new modules, threading, or GPUI-CE gotchas.
  Future sessions rely only on these docs, so write down what you learned.
