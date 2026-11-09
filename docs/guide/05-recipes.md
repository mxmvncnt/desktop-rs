# 5. Recipes: how to do the things you will do often

Every recipe says **which files you touch, in order**, and how long each step
takes. If a recipe touches a file in `workspace/` or `repo/`, it is usually
because of one of the remaining smells in
[cleanup-todo.md](cleanup-todo.md) — a feature change should ideally stay
inside its own `features/<feature>/` folder.

Conventions you will use everywhere (see [02-reading-rust-and-gpui.md](02-reading-rust-and-gpui.md)):

- Change state with `store.update(cx, |s, cx| ..)` and finish with `cx.notify()`.
- Git calls never run on the UI thread. Use `self.run(..)` for writes and
  `self.load(..)` for reads (in `repo/run.rs`). Copy the closest existing
  call instead of writing a new `background_spawn` by hand.
- Never hard-code colors. Use `Theme::of(cx)` and `theme.text`, `theme.bg`, …
  Check both `changes` and `changes-light` screenshots (see
  [06-build-test-debug.md](06-build-test-debug.md)).
- Long lists must be `uniform_list`. Rows in those lists are plain functions.

---

## Add a keyboard shortcut

**Files: `crates/app/src/actions.rs` + 1 handler file. ~10 min.**

1. Add the action name to the `actions!` list in `crates/app/src/actions.rs`
   (the list mirrors GitHub Desktop). Nothing fails yet: the action now exists
   but no key and no handler are attached, so steps 2 and 3 are required.
2. Add the key binding in the same file's `bind(cx)`, in the same style as the
   others:
   `KeyBinding::new(&format!("{M}-shift-x"), MyAction, Some(WORKSPACE))`
   (`WORKSPACE` and `M` — an alias for `platform::SECONDARY`, ctrl on Linux /
   cmd on macOS — are already in scope there).
3. Add the handler. Most actions are handled in `workspace/actions.rs`, which
   delegates to a feature:
   `on_action(cx.listener(|this, _: &MyAction, _, cx| this.my_feature_action(cx)))`,
   then implement `my_feature_action` in `features/<feature>/handlers.rs`
   (`impl Workspace`).
4. Document it in the README shortcuts table.

*Shortcut while a menu or dialog is open?* Don't add an action for it. Menus
get ↑/↓/Enter from the overlay stack (`workspace/overlay.rs`), and
`gd_ui::Dialog` already handles Enter/Esc even when a text input is focused
(see [03-how-it-works.md](03-how-it-works.md#menus-and-dialogs)).

## Add a menu item (right-click)

**Files: 3 in your feature folder. ~15 min. The cleanest recipe there is.**

Example: a new stash action. Look at `features/stashes/stash_menu.rs` and
`features/stashes/handlers.rs` to copy.

1. Add a variant to your feature's action enum
   (`features/stashes/state.rs`, e.g. `StashAction::MyAction`).
2. Add one row in the rows function (`stash_menu.rs`):
   `MenuRow::item(StashAction::MyAction, "Label", Icon::SomeIcon)`.
   Add the icon first if needed — see "Add an icon" below.
3. Handle the variant in `run_stash_action` (`handlers.rs`) with a `match`.
   Build errors will point you here if you forget a variant — that's the
   `match` doing its job (see 02).
4. If the action needs a git call, add a method in `features/stashes/ops.rs`
   (`impl RepoModel`) that calls `self.run(..)` / `self.load(..)`; the UI part
   never calls git directly.

Why this is nice: open/close, arrow keys, Enter, and rendering are all the
*one* generic `Menu` / `MenuView` (`components/menu_view.rs`). You don't
write any of that.

## Add a dialog (modal)

**Files: 5–6, spread across 4 places. ~45 min. This is the most expensive
common change — it is item 2 in [cleanup-todo.md](cleanup-todo.md).**

Example: the stash create/rename dialog (`features/stashes/stash_dialog.rs`).

1. Define the dialog enum in your feature's `state.rs`
   (`StashDialog { Create { input, .. }, Rename { stash, input } }`).
2. Add a variant to `Overlay` in `crates/app/src/workspace/overlay.rs`
   (e.g. `Stash(StashDialog)`), and an arm in `overlay_confirm`.
3. Render it in `crates/app/src/workspace/render.rs`, in the `overlays()`
   match — one arm that delegates to your feature's view function.
4. Build the dialog with `gd_ui::Dialog` (`crates/ui/src/components/dialog.rs`):
   it already handles Enter = confirm and Escape = cancel, even while a text
   input is focused. Put inputs in `Dialog::content`.
5. Add open/confirm/cancel methods in your feature's `handlers.rs`
   (`stash_confirm` is the model). Open with
   `self.open_overlay(Overlay::Stash(..), cx)`; never close other overlays by
   hand — `open_overlay` replaces the stack for you.
6. Add a screenshot view (see "Add a screenshot view" below) and run it.

## Add a background git command (read or write)

**Files: `crates/git/` + one `ops.rs` in the UI. ~30 min + test time.**

Reads (status, log, list branches) and writes (commit, checkout, push) are
both just "run a `git` command and parse the output". `crates/git/src/exec.rs`
is the one door: `Git::new(dir).args([..]).run()`.

1. **The git function.** Add (or extend) a function in `crates/git/src/` that
   shells out with machine-readable output only (`--porcelain`, `-z`,
   NUL-separated). Follow the closest existing file: `status.rs` for reads,
   `stash.rs` or `history_ops.rs` for writes. Set the env the same way
   `exec.rs` does (`LC_ALL=C`, `GIT_TERMINAL_PROMPT=0` for network).
   Parse in git's own separator format; never scrape human output.
2. **The test.** Add an integration test in `crates/git/tests/` using the
   helpers in `crates/git/tests/common/` (tempdir + `sh`). If you only
   verified it by hand against your own repo, it isn't tested yet.
3. **The UI caller.** In `crates/app/src/features/<feature>/ops.rs`
   (`impl RepoModel`):
   - write op →
     `self.run(Op::default(), |repo| repo.your_fn(..), |this, res, cx| { .. }, cx)`.
     Add `committing: true` only if the op creates a commit (that is what
     `commit` in `features/changes/ops.rs` does). For a long op that shows a
     busy banner, use `Op::labeled("Merging")` instead (one labeled op runs
     at a time). The fields are `label` / `committing` / `remote`, all
     optional (`repo/run.rs`).
   - read where only the latest result matters (you clicked A then B fast) →
     `self.load(|r| &mut r.some_slot, |repo| repo.your_fn(..), |this, res, cx| { .. }, cx)`.
     The first argument is a *closure to the field*, not the field, because
     `load` needs to keep the generation counter in it (`LoadSlot`).
   Copy `load_change_diff` (`repo/change_diff.rs`) for the `load` shape and
   `stash_push` (`features/stashes/ops.rs`) or `commit`
   (`features/changes/ops.rs`) for the `run` shape. Both are short and use
   the pattern exactly. `reload_stashes` in the same stash file is *not* an
   example: it calls `cx.background_spawn` by hand (because it reuses one
   repo handle rather than going through `run`).
4. After the op, `run` refreshes automatically (loads don't refresh; the data
   you loaded is usually part of a refresh anyway). Don't add your own
   refresh unless the data is not part of a refresh yet.

## Add a setting

**Files: `crates/app/src/features/settings/` + `settings.toml`. ~20 min.**

1. Add the field to `Settings` (`features/settings/model.rs`). The struct
   already has `#[serde(default)]`, so old `settings.toml` files without the
   key keep working (new fields need a `Default` value).
2. Add the control to the right page in `features/settings/` (`appearance_page.rs`,
   `diff_page.rs`, …). Use `CheckRow::new(id, label, checked, ..)` for on/off,
   `Select` for a dropdown (`appearance_page.rs`), `OptionRow` for a radio
   card, `Tabs` for a small choice — never a button whose label flips (see
   `docs/CONVENTIONS.md`).
3. In the setter: set the field, call `self.settings.save()`, call
   `cx.notify()`. Then apply it where it takes effect:
   `Workspace::set_color_theme` / `set_theme_mode` → `apply_theme`
   (`features/settings/handlers.rs`), or a field change →
   `apply_diff_options` (`features/diff/handlers.rs`).
4. Check both the dark and light snapshots: `changes` / `changes-light` for
   the main window, or `theme:<id>:dark` / `theme:<id>:light` for any theme
   the app knows (built-in or a user theme in the config dir).

## Add a row to a list

**Files: 2–3 in your feature folder. ~15 min.**

1. Pick the row style by list size:
   - **Short list** (stashes, ≤ 4 rows visible): a `RenderOnce` component is
     fine — copy `features/stashes/stash_row.rs` (`StashRow`), used inside
     `components::list(...)`.
   - **Hot list** (diff rows, the changes list — dozens of rows rebuilt every
     frame): a plain `fn row(..) -> AnyElement`, no component — every element
     costs frame time. Copy `features/changes/change_row.rs`.
2. Reuse `components/FileRow` / `components/BranchRow` if the row is a file
   or a branch. Only write a new row when those don't fit.
3. The list itself: `components::list(id, count, |range, w, cx| ..)` is the
   app's `uniform_list` with the standard inset. Anything that can be long
   must use it (or a raw `uniform_list` for the diff lists).
4. Clicks go through `Entity`s: `repo.update(cx, |r, cx| ..)` or
   `ws.update(cx, |this, cx| ..)`. Clone everything the closure needs before
   the `move` — that is the borrow-checker dance in 02.
5. Right-click: add the menu via the "Add a menu item" recipe above.
6. Keyboard navigation: wrap the list container with
   `nav_focus(div, NavList::X, ws)` (`workspace/navigation.rs`) so ↑/↓ drive
   it. If your list is not in `NavList`, you must add a variant there, a
   `select_*` method, and a `match` arm in `select_move` — that spread is a
   known smell ([cleanup-todo.md](cleanup-todo.md)).

## Add a screenshot view (so you can check your UI without a display)

**Files: 1 (3 if it is a brand-new feature). ~15 min. Do this for every
visual change.**

1. Add a static `View { name: "my-view", steps: &[...] }` to `views()` in
   `crates/app/src/features/<feature>/snapshot.rs`. A name ending in `*`
   matches by prefix. Each `step` is a plain fn that sets state directly
   (`step(ws, view, window, cx)`).
2. That's it for an existing feature — `crates/app/src/snapshot.rs` already
   collects every feature's `views()` in its `FEATURES` table. New feature:
   add `#[cfg(feature = "snapshot")] pub mod snapshot;` to its `mod.rs` and
   its `snapshot::views` to `FEATURES`.
3. Run: `cargo run --features snapshot -- --snapshot /tmp/desktop-rs/x.png <repo> my-view`
   (see [06-build-test-debug.md](06-build-test-debug.md)).

## Add an icon

**Files: 2. ~5 min.**

1. Copy an SVG in the same Solar style as the existing ones into
   `crates/ui/assets/icons/` (draw your own, or take one from the Solar icon
   set). The file name is what you reference in step 2.
2. Add one line inside the `icons! { .. }` list in `crates/ui/src/assets.rs`,
   with the file name **without** the folder and without `.svg`:
   `ArchiveUp => "archive-up-minimalistic",`.
   The macro line is what creates `Icon::ArchiveUp`; the string must match the
   file name. `rust-embed` embeds the folder, and a wrong/missing file name
   draws nothing (silently) — there is no compile-time check of the string.

## Add a color theme

**Files: 1. ~10 min.**

Copy `crates/ui/assets/themes/default.json` to
`crates/ui/assets/themes/<your-id>.json`, change `"id"` and `"label"`, and
fill **both** `"dark"` and `"light"` palettes (every family needs both).
Add user themes in `~/.config/desktop-rs/themes/*.json` instead of editing
the repo. Full format: `docs/THEMES.md`.
