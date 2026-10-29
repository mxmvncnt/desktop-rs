# Code conventions (app crate)

The goal is code a human can read in small pieces, organized like a Vue app.

## Write less code

- Minimal, easy-to-read code. A helper function, wrapper, struct or trait must earn its place:
  it removes real duplication (2+ call sites) or clearly makes the code easier to read.
  Otherwise inline it. No "just in case" generality, no pass-through layers, no comments that
  repeat the code.
- Before writing something, look for it: a crate (see Dependencies), an existing component in
  `components/` or `gd-ui`, an existing helper in `utils/`. Extend the existing one with a builder
  method instead of adding a near-copy.
- When you replace something, delete the old version in the same commit. Never leave both.
- One commit per issue, each one builds. Keep the 79 snapshots pixel-identical unless the change
  is meant to alter the look (then look at the images).
- Keep the `mod.rs` files; do not rename them to `foo.rs` + `foo/`.

## Feature folders

UI code lives in `crates/app/src/features/<feature>/`, one folder per feature
(stashes, history, merge, changes, diff, pickers, settings, welcome, ...).

```
features/stashes/
  mod.rs            // only `mod` lines + `pub use` of the public surface
  stash_panel.rs    // container component for the feature
  stash_row.rs      // small component, one per file
  stash_dialog.rs
  state.rs          // state structs + enums of the feature
  ops.rs            // RepoModel ops / background work of the feature
```

- Use `pub(super)` / `pub(crate)` to keep internals private. The rest of the app
  only sees what `mod.rs` re-exports.
- A component used by two or more features goes in `crates/app/src/components/`
  (`FileRow`, `FileList`, `FileDiffPane`, `BranchRow`, `DiffDisplayOptions`, `MenuRow`,
  `SuggestionCard`, `text_field`/`text_area`); a primitive with no app types goes in
  `gd-ui` (`ResizeHandle`, `OptionRow`, `CheckRow`, `Popover`, `modal`, `empty_state`,
  `Callback`). Non-UI helpers go in `crates/app/src/utils/` (`text.rs`, `paths.rs`). No `utils.rs`/`helpers.rs` grab bags:
  name a file after what it does (`relative_time.rs`).

## Components (Vue SFC ≈ one file)

| Vue | GPUI |
|---|---|
| stateless SFC with props | `#[derive(IntoElement)] struct X { .. }` + `impl RenderOnce` |
| SFC with `data()` | `Entity<X>` + `impl Render` |
| emit | `EventEmitter<E>` + `cx.subscribe`, or a callback field |
| store | `Entity<X Store>` per feature (`RepoModel`, `PopoverStore`, `AccountsStore`) or a `cx.global` |
| template | the `render()` fn |

- Default to `RenderOnce`. Only make an `Entity` when it owns state that
  outlives a frame (text input, scroll handle, open dialog, selection).
- **One `Render`/`RenderOnce` impl per file.** Small private
  `fn header(..) -> impl IntoElement` pieces are fine; once a piece needs its own
  props, it gets its own file.

## File layout: template first

1. The struct (props/state), short.
2. `impl Render` / `impl RenderOnce` (the "template").
3. `impl X { .. }`: handlers and logic.
4. Private helper fns.

## Stores

- One `Entity<XStore>` per feature that owns state with its own logic and several
  components (`RepoModel`, `PopoverStore`, `AccountsStore`). `Workspace` keeps the layout, the
  overlay stack and the list of stores; it `cx.observe`s a store to re-render.
- A component takes **its** store (`Entity<AccountsStore>`), calls `store.update(cx, |s, cx| ..)` and
  reads it in `render`, instead of receiving copies of its fields. Anything the store can't know
  (the open repository, the window) is a plain argument of the store method.
- Not a store: a struct of a few input handles that only `impl Workspace` handlers touch
  (`HistoryUi`, `StashUi`, `MergeUi`, `BranchUi`), a render cache mutated while painting (`DiffView`),
  or data read all over the app (`Settings`). An `Entity` there adds a layer without removing any
  `Entity<Workspace>` dependency.
- Group related `RepoModel` fields in a `#[derive(Default)]` struct (`ChangesState`, `HistoryLog`,
  `RefreshState`) so `empty()` stays short.

## Generic component, local override

A widget that appears in several places is one component in `components/` with the common
behavior and small builder methods for what differs (`BranchRow::icon/height/bright`,
`FileRow::leading`, `DiffDisplayOptions { sectioned }`). Don't copy it with edits.

## RenderOnce struct or plain fn

A `RenderOnce` struct is for a piece with its own props, callbacks or sub-pieces. A plain
`fn x(..) -> impl IntoElement` is enough for a stateless piece with one call site, and virtualized
rows (rebuilt every frame) are always plain fns. No struct whose `new()` only copies arguments.

## Functions and types

- Small functions (aim < 40 lines; render fns compose sub-pieces).
- Typed enums/structs instead of tuples, `bool` flags, or strings
  (e.g. `enum MenuRow { Item(MenuItem), Separator }`, not
  `Option<(MenuAction, &str, Icon, bool)>`).

## Tests

Unit tests live next to the source in a `tests/` folder of that source folder, one file per source
file named `<file>.test.rs` (`utils/text.rs` -> `utils/tests/text.test.rs`). The source file declares
them with `#[cfg(test)] #[path = "tests/text.test.rs"] mod tests;` and the test file starts with
`use super::*;` (private items stay reachable). Feature-gated tests keep their gate on the `mod`
line. Integration tests go in `crates/<crate>/tests/` (Cargo's own layout).

Tests that need a real `Workspace` (key dispatch, overlay stack: `workspace/tests/overlay.test.rs`,
the file watcher) use GPUI's headless test platform and are gated with
`#[cfg(all(test, feature = "snapshot"))]` on the `mod` line. Run them with
`cargo test -p desktop-rs --features snapshot`; plain `cargo test --workspace` skips them. They send
keys with `Window::dispatch_keystroke`, so bindings, key contexts and focused inputs are exercised.
Prefer a pure unit test on the state type (`components/tests/menu_view.test.rs`) when no window is needed.

## Dependencies

Prefer a well-maintained crate over hand-rolled platform/utility code
(config dirs, debouncing, serialization, time formatting, opening URLs, trash...).
Hand-roll only when a crate is too restrictive, and write down why.
Exception: all git access (reads, writes, network) shells out to the `git` CLI, never libgit2
(hooks, credential helpers, LFS, SSH behave exactly like the CLI). Reads parse machine-readable output.

## Feature folder example: stashes

`crates/app/src/features/stashes/` is the reference. Copy these decisions:

- `mod.rs` only has `mod` lines and `pub use` (panel, dialog view, menu rows,
  state types). Everything else is private to the folder.
- `state.rs`: `StashState` (model state, a field of `RepoModel`), `StashDialog`
  enum, `StashUi` (the dialog input; the dialogs are `Overlay::Stash`).
- `ops.rs`: `impl RepoModel` (background git work, generation counter).
- `handlers.rs`: `impl Workspace` handlers (open/close/confirm dialogs, menu
  actions). `pub(super)` for what sibling components call, `pub` only for what
  the rest of the app calls.
- Components are `#[derive(IntoElement)]` + `RenderOnce`, one per file
  (`stash_panel`, `stash_panel_header`, `stash_list`, `stash_row`,
  `stash_header`, `stash_file_list`, `stash_dialog`,
  ...), with typed props.
- Props: data they show (cloned, e.g. `Stash`) plus `Entity<RepoModel>` for
  model calls and `Entity<Workspace>` (`cx.entity()` at the call site) for UI
  actions. Handlers call back with
  `ws.update(cx, |this, cx| this.open_stash_dialog(..))` or
  `repo.update(cx, |r, cx| r.stash_pop(sha, cx))`; no `cx.listener` inside
  `RenderOnce`. Callback props are `Box<dyn Fn(&mut Window, &mut App)>`
  (`gd_ui::OptionRow`); name the type with `gd_ui::Callback`.
- Pieces that need `&mut Workspace` (the diff renderer owns scroll state) stay
  an `impl Workspace` method (`stash_detail.rs`) that composes the stateless
  components around it.
- No pass-through `new()`: when a constructor only copies its arguments into
  fields, make the fields `pub` and build the component with a struct literal
  at the call site. Keep `new` when it boxes a callback, computes something or
  returns `Option<Self>`. Stateless with one call site and no props struct: a
  plain `fn x(..) -> impl IntoElement` is enough.
- Panels that may be hidden expose `Component::new(..) -> Option<Self>`.
- Context menus return `Vec<MenuRow<A>>` with the feature's own action enum
  (`components/menu_row.rs`: `MenuRow::{Item(RowItem), Separator}`); `Menu::new` binds them to a
  `run_*_action` closure and `components::MenuView` renders them (see ARCHITECTURE.md, "Adding a menu").
- Dialog variants are built by one small method per variant on the
  `RenderOnce` struct, not one giant match.
- Every menu has its own action enum (`MenuAction`, `StashAction`, `HistoryAction`, ...); rows are built in
  one small fn per group (`commit_menu_rows.rs`).
- Identical rows across features live in `components/` (`FileRow` with an id
  namespace, `.leading(..)` slot and callbacks; `BranchRow` with trailing children).
- Replace `(bool, bool)` click flags with an enum (`ClickKind`).
- Menus and dialogs are variants of `workspace::Overlay` (one stack on `Workspace`), never
  per-feature `Option` fields; Esc/Enter/arrows are routed once in `workspace/overlay.rs`. Typed structs
  replace tuples (`branches::RemoteBranch`, not `(String, String)`).

## Feature folder example: diff (hot rows)

`crates/app/src/features/diff/` has subfolders per sub-feature, each with a `mod.rs` that only lists
what the siblings use:

```
diff/
  view.rs         Workspace::render_diff (entry point of every pane) + DiffFrame
  state.rs        DiffView, DiffPane, gutter pointer state
  handlers.rs     impl Workspace setters and horizontal scroll
  highlight.rs    syntax runs, line_text
  list/           unified.rs split.rs (lists), unified_rows.rs split_rows.rs (hot rows), split_model.rs, rows.rs, text_flow.rs
  gutter/         number_cell.rs, block_handle.rs (RowSelect), line_selection_store.rs (RepoModel ops), style.rs
  text_selection/ model.rs (TextSel), mouse.rs (TextSelect, per-row binding), handlers.rs (impl Workspace)
  expand/         buttons.rs (Expander + ExpandButton)
  options/        button.rs menu.rs whitespace_note.rs
  snapshot.rs     snapshot views
```

Decisions:

- **Allowed exception: hot rows stay plain fns/methods.** Virtualized rows are
  rebuilt for every visible row every frame, so `UnifiedRows::row`,
  `SplitRows::row`, `number_cell` and `RowSelect::handle_cell` are not
  `RenderOnce` structs (no per-row props struct, no extra boxing). Per-list and
  per-frame pieces are components (`UnifiedList`, `SplitList`, `DiffFrame`,
  `DiffOptionsMenu`, `DiffOptionsButton`, `WhitespaceNote`,
  and `ExpandButton`, which only appears on hunk headers).
- Row renderers borrow their shared inputs in one small struct
  (`UnifiedRows { diff, hl, sel, exp, .. }`) instead of 8 positional args.
- Keep the perf tricks when touching this folder: `Arc::ptr_eq` caches
  (`split_for`, `wrap_list_for`), `restrict_scroll_to_axis` and the manual
  h-scroll in `list/rows.rs`/`text_flow.rs`, and no `h_full` on gutter cells.
- Typed replacements: `DiffPane` (not `commit: bool`), `BlockRow::{First,
  Rest}`, `HoveredHandle`/`GutterDrag` (not tuples).

## Feature folders: changes, pickers, settings, menus, welcome

- `features/changes`: `Sidebar` (tabs + `ChangesPanel` or `CommitList`),
  `ChangesPanel` composes `ChangesHeader`, `ChangesList`, `StashPanel`,
  `UndoStrip`, `CommitBox` (+ `AmendNotice`). `Sidebar::new(ws, repo, cx)` and
  `ChangesPanel::new` read the workspace once and hand typed props down;
  derived values live in `state.rs` (`SelectionSummary`, `CommitButton`, instead
  of a dozen locals). The main pane (`render_changes_main`) stays an
  `impl Workspace` method (diff renderer needs `&mut Workspace`). Hot row:
  `change_row` is a plain fn. Changes-specific `RepoModel` ops (include/exclude,
  commit, ignore, discard, undo) are in `ops.rs`; `select_change` stays in
  `repo/change_diff.rs` because it drives the private diff loader.
- `features/pickers`: `PickerShell` owns filter input, empty state and
  Enter/up/down capture; `RepoPicker`/`BranchPicker` pass an `on_enter` callback.
  `Workspace::repo_picker()/branch_picker()/new_branch_form()/clone_form()`
  (handlers.rs) clamp the cursor and build the components. `PopoverStore` (`store.rs`,
  `Workspace::pickers`) owns the cursor, the three inputs and the new-branch base; the
  picker components take `Entity<PopoverStore>` for stepping/toggling and a plain
  callback for Enter, and keep `Entity<Workspace>` only for actions that open repositories. Hot rows `repo_row`/`branch_row` are plain fns.
- `features/settings`: also owns the persisted `Settings` model (`model.rs`).
  `SettingsModal::open(ws, cx)` -> `Option`, one `RenderOnce` per page.
- `features/menus`: the Changes file/header menus (`MenuAction`) and the discard
  dialog (`DiscardDialogView`).
- `utils/text.rs`: `plural`, `split_path`, `sanitize_branch_name`, `relative_time`,
  commit authors. `utils/paths.rs`: `file_url`, `reveal`, `copy_path`.
- Closure-heavy `RenderOnce` components call back through `Entity<Workspace>`
  (`ws.update(cx, |this, cx| ..)`), never `cx.listener`.
- Return types that borrow `&self` need `+ use<>` (edition 2024).
