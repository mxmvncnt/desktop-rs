# 8. Find the code behind what you see, and debug it

This is the "I see a thing on screen, where is it?" guide, plus the debugging
tools that actually exist in this repo. Keep it open while you work.

Everything here assumes you are in the repo root and that scratch files go in
`/tmp/desktop-rs/` (see `docs/AGENTS.md`).

---

## The 30-second version

| You see | First thing to search |
|---|---|
| Text | a **fragment** of the text: `rg -n "Wrap long lines" crates/` |
| An icon | the icon name: `rg -n "Icon::ArchiveUp" crates/` |
| A row in a list | the model field the row reads (`stashes.list`, `r.work.status`) or `nav_focus` |
| A menu / dialog / popup | `crates/app/src/workspace/overlay.rs` (the `Overlay` enum), then the type it holds |
| A whole page | `crates/app/src/workspace/render.rs` → `Workspace::body` and `Workspace::overlays` |
| A color / size / font | `crates/ui/src/theme.rs` (tokens) and `crates/ui/assets/themes/*.json` |
| Still lost | the feature's `snapshot.rs`: the view that looks like your screen sets the exact fields that matter |

Two warnings that save time:

- Text built with `format!` is **not** searchable as it appears. The undo strip
  renders `format!("Committed {}", relative_time(c.time))`
  (`features/changes/undo_strip.rs`), so search `"Committed "`, not
  "Committed just now".
- Methods on the two big objects are scattered. `Workspace` methods live in
  **39 files** and `RepoModel` methods in **19 files**. Always search the whole
  crate for `fn name`, never assume it is next to the struct (see
  [02-reading-rust-and-gpui.md](02-reading-rust-and-gpui.md)).

---

## Worked example: the "Committed …" strip with the Undo button

This is the full chain, with the real commands. Line numbers drift; the
function names don't.

1. **The text.** `rg -n 'Committed \{' crates/`
   → `features/changes/undo_strip.rs`, `UndoStrip::render`. It renders
   `Committed {relative_time}` and a `Button` labelled `Undo` calling
   `Workspace::undo_commit`.
2. **Who builds the strip?** `rg -n "UndoStrip" crates/`
   → `features/changes/changes_panel.rs`, `ChangesPanel::new`:
   `let last = r.undoable_commit()...; undo: last.map(|c| UndoStrip { .. })`.
   So the strip exists only when `RepoModel::undoable_commit()` returns
   something (`repo/state.rs`), which hides it after a branch switch.
3. **The click.** `rg -n "fn undo_commit" crates/`
   → `workspace/repo_commands.rs`: `self.with_repo(cx, |r, cx| r.undo_last_commit(cx))`.
   `with_repo` is the "if a repo is open, update it" idiom; use it instead of
   repeating `if let Some(repo) = &self.repo`.
4. **The git work.** `rg -n "fn undo_last_commit" crates/`
   → `features/changes/ops.rs`: it calls
   `self.run(Op::default(), |r| r.undo_last_commit(), ..)`. The `|r|` closure
   is the *git* half (blocking, runs off-thread); the `move |this, _, cx|`
   closure is the UI half. It emits `RepoEvent::CommitUndone`.
5. **Back to the UI.** `rg -n "CommitUndone" crates/`
   → `workspace/repos.rs`, `Workspace::on_repo_event`: it calls
   `commit_box.set_text(summary, description, cx)`, so the message reappears
   in the commit box.

That five-step loop — **view → who builds it → click handler → git op →
event/subscription** — is the same for almost every feature.

---

## Anchors that exist everywhere (good search terms)

| Anchor | Example | Why it helps |
|---|---|---|
| `#[derive(IntoElement)]` + `impl RenderOnce` | `pub struct StashRow` | the struct name is usually the on-screen widget name |
| element id | `.id("changes-header")`, `.id(("stash-title", stash.index))` | ids are stable literals; `rg -n '\.id\(' crates/app/src` lists them all |
| action enum variants | `StashAction::Apply`, `HistoryAction::Revert` | find the menu rows and the handler with one word |
| `MenuRow::item(.., "Label", Icon::X)` | `features/stashes/stash_menu.rs` | the one-line row definition |
| overlay types | `Overlay::Stash(StashDialog)` | menus/dialogs are enumerated in `workspace/overlay.rs` |
| list + nav wiring | `nav_focus(div(), NavList::Stashes, ws)`, `components::list(` / `uniform_list(` | tells you the list container and keyboard target |
| background call | `self.run(` (writes), `self.load(` (reads) | most git work in the UI goes through these two; refresh, history paging, background fetch and CI HTTP spawn directly |
| setting key | `diff_wrap` in `settings.toml` = field in `Settings` | finds the model field, the setter, and the page |
| icon | `Icon::ArchiveUp` | finds every use of that glyph |
| snapshot view | `View { name: "stash-detail", .. }` | a state you can reproduce headlessly |

A typical search sequence when you have nothing but a screenshot:

```sh
rg -n "the text you see" crates/            # 1. the label
rg -n "TheComponentName" crates/            # 2. who builds it
rg -n "fn the_handler" crates/              # 3. what the click does
rg -n "fn the_git_op" crates/               # 4. the git half
rg -n "TheEvent" crates/                    # 5. who reacts afterwards
```

If the text is dynamic, search the *model field* instead (`rg -n
"relative_time" crates/app/src/features/changes`).

---

## Debugging tools that exist

### 1. Logs (fastest, works everywhere)

The app logs to stderr with the `log` crate. Default filter:
`warn,desktop_rs=info,gd_git=info` (`crates/app/src/main.rs`).

```sh
RUST_LOG=desktop_rs=debug cargo run            # + refresh lines and app debug logs
RUST_LOG=gd_git=debug cargo run                # + every git invocation
```

There are only ~10 `log::` calls in the whole app, so for your own
investigation add a temporary line and remove it before committing:

```rust
log::debug!("select_change: ix={ix} file={:?}", file.path);
// or, inside a background closure:
eprintln!("stash_list -> {} entries", list.len());
```

`dbg!(value)` also works and prints the file/line; it needs `Debug` on the
value (most state structs derive it). If the app panics, `RUST_BACKTRACE=1
cargo run` prints the Rust stack trace instead of just the message.

### 2. A real breakpoint (`rust-gdb` is installed on this machine)

```sh
cargo build                                   # dev profile, debug info on
rust-gdb --args target/debug/desktop-rs /tmp/desktop-rs/demo
(gdb) break crates/app/src/features/changes/ops.rs:142
(gdb) run
(gdb) bt                                      # who called it
(gdb) print ix                                # locals in scope
(gdb) continue
```

Useful spots to break: the click handler, `RepoModel::run` / `RepoModel::load`
(`repo/run.rs`), `RepoModel::request_refresh` (`repo/refresh.rs`), and
`Git::run` (`crates/git/src/exec.rs`) if you think a git command is wrong.

For the headless harness, build the snapshot binary first and debug the same
path:

```sh
cargo build --features snapshot
rust-gdb --args target/debug/desktop-rs --snapshot /tmp/desktop-rs/x.png /tmp/desktop-rs/demo changes
```

If you break inside GPUI or on an async task, `bt` will show a long chain;
the app frames still have readable names like `desktop_rs::features::…`.

### 3. Reproduce the state without a display (snapshot views)

The fastest way to inspect a rare state (a dialog with a specific error, a
mid-merge banner) is to add a throwaway `View`:

```rust
// crates/app/src/features/<feature>/snapshot.rs, inside views()
View { name: "debug-my-state", steps: &[debug_my_state] },

// and below
fn debug_my_state(ws: &mut Workspace, _view: &str, _w: &mut Window, cx: &mut Context<Workspace>) {
    if let Some(r) = ws.repo.clone() {
        let r = r.read(cx);
        eprintln!("tab={:?} selected={:?}", r.tab, r.work.selected);
    }
    // set the state you care about, e.g. ws.overlays.push(Overlay::Stash(..));
}
```

```sh
cargo run --features snapshot -- --snapshot /tmp/desktop-rs/x.png /tmp/desktop-rs/demo debug-my-state
```

The `eprintln!` appears in the terminal; the PNG shows the result. Three
extras:

- `GD_SNAPSHOT_MOUSE=x,y` parks the pointer (hover styles, `on_hover`).
- `GD_SNAPSHOT_DRAG=x1,y1,x2,y2` does a real left-button drag (text
  selection, gutter line selection).
- `crate::snapshot::press_next(Some(NavList::Changes), ws, window, cx)` is a
  helper that focuses a list and dispatches a real `SelectNext` action — use
  it to debug ↑/↓ behaviour instead of calling `select_*` directly.

Remember to delete the debug view (or keep it if it is genuinely useful) and
to keep the other snapshots pixel-identical.

### 4. Headless tests that press real keys

`crates/app/src/workspace/tests/overlay.test.rs` builds a temporary repo,
opens a real `Workspace` on GPUI's test platform and sends
`Window::dispatch_keystroke` — so key bindings, key contexts and
Enter/Esc capture are exercised. Run them with:

```sh
cargo test -p desktop-rs --features snapshot overlay
```

Copy that fixture when your bug is "the key does the wrong thing". This is
better than a snapshot view for anything that involves focus.

### 5. Performance and idle behaviour

```sh
GD_DEBUG_FRAMES=1 cargo run                    # renders=N moves=M once per second
```

Silence = truly idle. Many renders with few pointer moves = something
notifies or animates on its own. For scroll cost, use `GD_SNAPSHOT_BENCH`
(see [06-build-test-debug.md](06-build-test-debug.md)).

### 6. Refresh loops

If the app feels busy for no reason:

```sh
RUST_LOG=desktop_rs=debug cargo run
```

`repo/refresh.rs` logs `refresh #N (status|full)` per refresh. A steady
stream means a loop; the usual causes are the watcher reacting to git's own
reads (`Access(Open)` events are filtered — keep that filter) or a `notify`
inside a refresh.

---

## Inspecting state from inside a handler

- Reads and updates cannot overlap. `let r = repo.read(cx);` then calling
  `repo.update(cx, ..)` will panic (borrow). Print, drop the read, then update.
- To reach the model from a `Workspace` method:
  `let Some(repo) = self.repo.clone() else { return; };` then
  `repo.read(cx)` / `repo.update(cx, |r, cx| ..)`. `self.with_repo(cx, |r, cx| ..)`
  is the shorter form for updates.
- `cx.entity()` inside `Workspace` gives the `Entity<Workspace>` callbacks
  need; `cx.notify()` is what makes the frame redraw.
- Most state is grouped: `r.work.*` (changes), `r.log.*` (history),
  `r.sync.*` (refresh), `r.stashes`, `r.pr`, `r.merge`. Start there instead of
  grepping the top-level struct.

---

## Symptom → likely cause

| Symptom | Look at |
|---|---|
| A click does nothing | the `on_click` is inside an `occlude()`d backdrop; the element is disabled; or the handler updates a store nobody observes |
| The value changed but the screen didn't | missing `cx.notify()`; inside a cached pane, the `Workspace` must notify (see `workspace/init.rs`'s `cx.observe(&store, ..)`) |
| Old data flashes / wrong file's diff | `LoadSlot` generation: a normal slot drops stale results, while `LoadSlot::late()` deliberately applies them (history paging); see `repo/run.rs` |
| Nothing refreshes after a change | `request_refresh` not called; or a `Status` refresh when you needed `Full` |
| Colors wrong in light mode | hard-coded color instead of a theme token; check `theme.ink()` and run the `-light` snapshot |
| Popup in the wrong place | window-coordinate mouse positions need the `client_inset` subtracted (`platform/linux/chrome.rs`) |
| Typing is stolen by shortcuts | a focused input binds ↑/↓ itself; dialogs must use `capture_action` |
| App freezes | a blocking call on the UI thread (git, HTTP, `std::thread::sleep`) — move it to `cx.background_spawn` |
| Panic on startup | a bad embedded theme JSON (the registry panics on a broken built-in theme); a missing icon does **not** panic, it draws nothing |
| Menu/dialog flashes | overlay entrance animation; use `motion::handoff` (GPUI-CE gotcha) |

Deeper causes and GPUI-CE quirks are in `docs/ARCHITECTURE.md`
§"GPUI-CE gotchas". Read that before fighting the framework.

---

## Rules for debug code

- Delete temporary `dbg!`/`eprintln!`/debug views before committing, unless
  they are actually useful (then give them a real name and a comment).
- Never debug against your real repository or real config: run with
  `GD_CONFIG_DIR=/tmp/desktop-rs/config` and a scratch repo in
  `/tmp/desktop-rs/`.
- After a UI change, take the relevant `--snapshot` PNG and look at it. A
  change that "obviously works" is how most regressions got in.
- When you learn a non-obvious cause, write it down in
  `docs/ARCHITECTURE.md` (gotchas) or here. The next person has the same bug.
