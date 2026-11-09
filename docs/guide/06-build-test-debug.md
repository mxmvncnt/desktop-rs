# 6. Build, test, debug and release

Keep this file open on a second screen while you work. Every command assumes
you are in the repo root (`/home/maxime/Git/desktop-rs` on this machine).

> Rule from `docs/AGENTS.md`: scratch repos, screenshots and logs go in
> `/tmp/desktop-rs/`, never elsewhere in `/tmp`, never in `~`. Always run the
> app with `GD_CONFIG_DIR=/tmp/desktop-rs/config` so tests never overwrite
> your real `~/.config/desktop-rs/settings.toml`.

## The daily loop

```sh
cargo run                                            # build + run the app
cargo build                                          # just build (fast check)
cargo build --features snapshot                      # also build the screenshot mode
cargo clippy --workspace --all-targets               # lints; must be warning-free
cargo test --workspace                               # unit + integration tests
```

If you changed anything in `crates/app/src/features/*/snapshot.rs`, or
renamed anything, you **must** also build with `--features snapshot` — the
snapshot modules only compile with that feature flag.

One change, one commit, each commit builds. Keep screenshots pixel-identical
unless the change is meant to alter the look — then look at the images (next
section) and commit them too.

## Take screenshots (no display needed)

```sh
mkdir -p /tmp/desktop-rs
cargo run --features snapshot -- --snapshot /tmp/desktop-rs/shot.png <path-to-a-test-repo> changes
```

`<path-to-a-test-repo>` is any git repo (it needs a commit; add uncommitted
changes to see the Changes pane). For `welcome` and for any view whose name
ends in `-norepo`, the path is ignored — pass `.` as a placeholder. The last
argument is the view name
(`changes`, `history`, `repos`, `branches`, `new-branch`, `clone`, `welcome`,
`icons`, `changes-light`, `theme:nord:dark`, or one of the names from any
`features/<feature>/snapshot.rs` — a name ending in `*` matches a prefix).
An unknown name renders the plain workspace with no extra steps, which is why
`changes` and `welcome` work without having a `View` of their own.

Then open `shot.png` and look at it. Useful extra flags (environment
variables):

| Variable | What it does |
|---|---|
| `GD_SNAPSHOT_MOUSE=150,60` | move the mouse there before capturing (tests hover states) |
| `GD_SNAPSHOT_DRAG=x1,y1,x2,y2` | drag with the left button |
| `GD_SNAPSHOT_SIZE=2560,1440` | window size in pixels |
| `GD_SNAPSHOT_BENCH=300` | send 300 wheel steps at the mouse, print `p50 / p90 / max` draw ms (needs `GD_SNAPSHOT_MOUSE`) |
| `GD_SNAPSHOT_CSD=1` | force the Linux client-side decorations frame |

`GD_SNAPSHOT_BENCH` is how you check scroll performance: on a 165 Hz screen a
frame over 6.06 ms misses a vblank and scrolling *feels* half as fast. It only
runs when both it and `GD_SNAPSHOT_MOUSE` are set, and it times
`window.draw` (render + layout + prepaint + paint, CPU only, no GPU). See
`docs/ARCHITECTURE.md` §Performance for the numbers we have.

If the app won't start headless in your environment, use
`import`/`grim` on a real display — but on this machine they don't exist,
so `--snapshot` is the only way.

## Make a test repository

```sh
mkdir -p /tmp/desktop-rs/demo && cd /tmp/desktop-rs/demo
git init -b main && git config user.email a@a.a && git config user.name A
echo hello > a.txt && git add . && git commit -m init
echo changed >> a.txt && echo new > b.txt
```

Now `cargo run --features snapshot -- --snapshot /tmp/desktop-rs/x.png /tmp/desktop-rs/demo changes`
shows a repo with one modified and one untracked file. Create branches /
commits / stashes
(`git stash push -m "wip"`, `git checkout -b feature`, …) to test those
screens. Never use your real repo for snapshot experiments — refreshes and
ops *write* to it.

## Run the tests

```sh
cargo test -p gd-git        # git layer only (fast, no UI)
cargo test -p gd-forge      # forge/API client only
cargo test -p desktop-rs --features snapshot   # the tests that need a window
```

- Unit tests live next to their source: `foo.rs` → `foo/tests/foo.test.rs`,
  loaded with `#[cfg(test)] #[path = "tests/foo.test.rs"] mod tests;`.
  The test file starts with `use super::*;` so it can reach private items.
- Integration tests that need a real temporary repository live in
  `crates/git/tests/` (see `crates/git/tests/common/`).
- Four test files need a **headless GPUI window** and only compile/run with
  the snapshot feature: the file watcher
  (`crates/app/src/tests/watcher_tests.test.rs`), the overlay keyboard tests
  (`crates/app/src/workspace/tests/overlay.test.rs`), the commit-box typing
  test (`features/changes/tests/commit_box.test.rs`) and the gutter line
  selection test (`features/diff/gutter/tests/line_selection_store.test.rs`).
  Their `mod` lines carry `#[cfg(all(test, feature = "snapshot"))]`, so plain
  `cargo test --workspace` skips them on purpose. Run one with, e.g.,
  `cargo test -p desktop-rs --features snapshot watcher`.

If a test fails, read the compiler/test output top-down: the first error is
almost always the real one; the rest is fallout.

## Logging and frame debugging

```sh
RUST_LOG=info,desktop_rs=debug cargo run
```

Or the debug helpers built into the app:

- `GD_DEBUG_FRAMES=1` prints `renders=N moves=M` once a second while the
  window redraws (`crates/app/src/frame_debug.rs`). Silence = truly idle.
  Lots of renders but few pointer moves = something re-renders on its own
  (a `cx.notify()` in the wrong place, or an animation using `repeat()` —
  the app uses one-shot animations only, except the toolbar busy pulse).
- `GD_CONFIG_DIR` (already mentioned) relocates the config folder.

The default log filter is `warn,desktop_rs=info,gd_git=info` (`main.rs`).
`RUST_LOG=desktop_rs=debug` adds `refresh #N (status|full)` lines
(`repo/refresh.rs`), so a refresh loop is visible as an endless stream of them.
There are only ~10 `log::` calls in the whole app, so if you need to see what
a function is doing, add your own temporary `log::debug!(..)` / `eprintln!(..)`
and remove it before you commit.

## Release

Releases are Flatpaks built by GitHub Actions (`.github/workflows/` +
`build-aux/flatpak/`). To tag a new release:

```sh
./bump-version.sh <version>   # sets version, commits, tags
```

Then publish a GitHub release. That is the workflow's only trigger: it builds
the Flatpak, attaches bundles, and deploys the Flatpak repository to GitHub
Pages. No tests run automatically. The vendored GPUI
patches in `vendor/` sit on top of the `gpui-ce` git `rev` pinned in the root
`Cargo.toml` (the `[patch."https://github.com/gpui-ce/gpui-ce"]` section) —
bump the rev deliberately and re-check `docs/ARCHITECTURE.md` §GPUI-CE pin.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `cargo build` fails with a long list of errors | You added an enum variant and didn't handle it in every `match` | Read the first error; the compiler lists every `match` to fix |
| Build fails only without `--features snapshot` (or only with it) | `#[cfg(feature = "snapshot")] pub mod snapshot;` missing or a rename broke snapshot code | `cargo build --features snapshot` to see the errors |
| The screen doesn't update after your change | You changed a field and forgot `cx.notify()` | Add it; see 02 "Things that will confuse you" |
| The app writes to your real settings | You ran without `GD_CONFIG_DIR` | Stop, delete the damage, always run with `GD_CONFIG_DIR=/tmp/desktop-rs/config` |
| Scrolling feels laggy | Frame time > 6 ms at 165 Hz | Re-run the `GD_SNAPSHOT_BENCH` command; look for heavy rows (every element in a diff row costs ~0.35 ms across visible rows) |
| Snapshot PNG is black / transparent | Animation captured at opacity 0, or headless GPU not available | The harness already sleeps; if it persists, check that you built with `--features snapshot` |
| `cargo clippy` complains about `type_complexity` / `collapsible_if` | A long tuple or a nested `if` clippy can flatten | Fix it as asked, or add `#[allow(..)]` with a one-line reason. The tree is clean today except one warning inside `vendor/`, so any new one is yours |

## If you're stuck

1. Search the on-screen text: `rg "Undo" crates/`. Most labels are written
   literally in the code — but text built with `format!("Committed {}", ..)`
   is not, so search a stable fragment (`"Committed "`), not the whole line.
2. `rg "fn name_you_look_for" crates/` — remember every `impl RepoModel` /
   `impl Workspace` block can live in any feature folder (see 02).
3. Read `docs/ARCHITECTURE.md` for the deep dive on the topic (threading,
   GPUI-CE gotchas, diff engine, partial staging). It is the reference; this
   folder is the map.
4. [08-find-and-debug.md](08-find-and-debug.md) has the full technique:
   visual element → file, plus how to inspect state and set a breakpoint.
