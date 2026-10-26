# Code base cleanup report (2026-10-25)

> **Decision (2026-10-25): we keep our own components; Part 3 (gpui-component) is not adopted.**
> The component cleanup follows Part 1 §1.1/§1.5–1.7 and Part 2 §P2.10 instead. Part 2's non-UI
> library swaps (git2, watcher, serde, keyring, url, …) still apply. P2.2 (syntect) stays optional.
> Part 1 removes copies inside our own code. Part 2 removes our own code entirely by
> handing it to well-known libraries, drops edge cases that never happen, and defines the
> `components/` folder. Do Part 2's replacements before Part 1's dedup where they
> overlap: there is no point deduplicating a parser that git2 will delete.

# Part 1: duplication and structure

What this is: a review of every Rust file in `crates/` (about 23k lines in about 230
files). It looks for code smells, duplication, oversized or over-split files, useless helpers,
and patterns that are too complex for what they do. Every finding gives the location, why
it hurts a human reader, and the simplification. Where possible it also gives **what
disappears**, because many of the fixes are "make X generic, then delete 3 copies".

The target is the Vue composition-API layout you described:

| Vue (composition API) | desktop-rs equivalent to aim for |
|---|---|
| one store per feature (Pinia) | one `Entity<FeatureStore>` per feature, **not** fields on `Workspace` |
| SFC: one template + its logic | one `Render`/`RenderOnce` per file, handlers in the same file |
| composables (`useX()`) | generic helpers/structs in `shared/` (`MenuState<A>`, `DiffDetail`, `spawn_load`) |
| generic component + props/slots | one generic component with builder overrides (`FileList::new(..).id("pr")`) |
| feature folders | `features/<feature>/…` (already done) |

Status quo in one paragraph: the **folder layout already looks like Vue**, but the **data
flow doesn't**. Nearly all state still lives in two god objects (`Workspace` and
`RepoModel`). Every component receives `Entity<Workspace>` (87 fields), and every "store"
is a plain struct bolted onto those two objects. The same widget or flow was often
re-implemented per feature instead of being made generic: 4 menu systems, 4 file-detail
panes, 4 branch rows, 3 op runners and 3 highlight loaders. Most of the reading burden
comes from those copies, not from any single hard algorithm.

Sanity check: `cargo clippy --workspace --all-targets` reports only 17 warnings
(complex types, collapsible `if`s, `toolbar/toolbar.rs` module inception, a large enum
variant in `changes/sidebar.rs`). The problems below are structural, so lints don't catch
them.

---

## 0. Top 10 by payoff (do these first)

| # | Change | Rough lines removed | Concepts removed |
|---|---|---|---|
| 1 | One generic **overlay/menu store** instead of 5 menu systems + 9 overlay flags (§1.1) | ~450 | 4 `*MenuView`, 4 `menu_entry`, 5 `*_step`, 4 confirm routers, `overlay_open`, `dismiss_top_overlay` chain |
| 2 | One **`DiffDetail` store** (files + selected file + diff + highlight + generation) for commit / stash / PR / changes (§1.3) | ~350 | 3 highlight spawners, 3 `HighlightCache`s, 3 `select_*_file` copies, 3 file-list components, 3 detail panes |
| 3 | Split `Workspace` into **per-feature stores** (§1.2) | ~0 net, but 87 `Entity<Workspace>` props go away | the god object |
| 4 | One **op runner** (`run`) instead of `run` / `run_op` / `hist_run` + 20 hand-written spawn blocks (§1.4) | ~200 | 3 runners, repeated `error = Some(e.to_string())` |
| 5 | **serde + toml** for settings/accounts (§5.9) | ~150 | 4 file formats, hand parsers, `Display`/`FromStr`/`key()`/`from_key()` |
| 6 | Generic **`BranchRow`**, **`FileList`**, **`ResizeHandle`**, **`TextArea`**, **`Popover`** components (§1.6) | ~300 | 12 near-identical components |
| 7 | Git crate: one `blob_sides()`, one `stage_and_commit()`, one `count()`, one `parse_name_status()` (§2) | ~150 | duplicated commit/diff plumbing |
| 8 | Delete dead / write-only code (§6) | ~80 | `loaded_base`, `for_mode`, `TEXT_*`, `BUBBLE_RADIUS`, `hide_cancel`, … |
| 9 | Turn `snapshot.rs`'s 457-line `match` into a table plus per-feature setup fns (§7) | ~0 net | the longest function in the repo |
| 10 | Fix doc/comment drift (§8) | — | wrong pointers that send readers to files that don't exist |

---

## 1. Architecture-level problems

### 1.1 Five menu systems and nine overlay flags, routed by hand

**Where**
- State is spread over: `Workspace.menu`, `.dialog`, `.popover`, `.settings_open`,
  `.diff_view.menu`, `.history_ui.{menu,dialog}`, `.stash_ui.dialog`,
  `.merge_ui.{menu,dialog}`, `.branch_ui.{menu,dialog}`, and `RepoModel.pr.base_menu`.
- "Is anything open?": `workspace/actions.rs:62` `overlay_open()` ORs 8 conditions, plus
  `MergeUi::wants_menu_context` (`merge/state.rs:114`) and `BranchUi::wants_menu_context`
  (`branches/state.rs:69`).
- Esc: `workspace/actions.rs:82` `dismiss_top_overlay` is a 10-branch `if/else if` chain
  with empty `{}` bodies that only exist for their side effects.
- Enter: `menus/key_routing.rs:32` `menu_confirm` → `stash_confirm` / `conflict_menu_confirm`
  / `branch_ui_confirm` / `confirm_dialog` / `run_menu_action`, plus
  `workspace/actions.rs` `hist_confirm_key` first.
- Arrow keys: **five copies of the same loop**: `menus/key_routing.rs:10` `menu_step`,
  `history/handlers.rs:31` `hist_menu_step`, `branches/handlers.rs:45`
  `branch_menu_step`, `merge/conflict_handlers.rs:27` `conflict_menu_step`, plus
  `pickers/handlers.rs:85` `step_picker` and `pickers/popover_handlers.rs:84`
  `step_popover_selection` (**byte-for-byte identical bodies**).
- Rendering: four `*MenuView` components (`menus/context_menu_view.rs`,
  `history/commit_menu.rs`, `branches/branch_menu.rs`, `merge/conflict_menu.rs`). Each has
  its own copy of `fn menu_entry(row, ws) -> MenuEntry`, and these copies differ only in
  which `run_*_action` they call.
- Z-order: `workspace/render.rs:90` `overlays()` lists 13 overlay builders by hand.
- Two row types for one thing: app `shared/menu_row.rs` `MenuRow/RowItem` and gd-ui
  `MenuEntry/MenuItem` have the same fields (label, icon, danger, disabled). The only
  difference is that one carries an action enum and the other a callback.
- Copy-paste bug smell: `BranchMenuView` uses the element id `("merge-menu", gen)`
  (`branches/branch_menu.rs:36`), the same id as `ConflictMenuView`.
- Opening one overlay has to close the others by hand. That's why you see
  `self.popover = None; self.menu = None; self.dialog = None; self.merge_ui.menu = None;
  self.close_branch_overlays();` repeated in `open_merge_dialog`, `open_commit_menu`,
  `open_stash_dialog`, `open_switch_dialog`, `open_menu`, `open_diff_options`, …

**Why it hurts.** Adding one menu means touching 6 files. To answer "what does Esc do?" a
reader has to follow 10 functions across 6 features. Ordering bugs (which overlay wins)
are invisible.

**Simplification (Vue: one overlay store + one generic `<ContextMenu>`):**
```rust
// shared/overlay.rs (the store)
pub enum Overlay {
    Menu(MenuState),                 // any context menu
    Dialog(Box<dyn DialogSpec>),     // or an enum of feature dialogs
    Popover(Popover),
    Settings,
}
pub struct MenuState { pub position: Point<Pixels>, pub selected: Option<usize>, pub rows: Vec<MenuRow<AnyAction>> }
impl MenuState { pub fn step(&mut self, delta: isize) { /* the one loop */ } pub fn confirm(&self) -> Option<AnyAction> }
```
- `Workspace` (or an `OverlayStore` entity) holds **one** `Option<Overlay>`, or a `Vec`
  if stacking is really needed. Opening something replaces it, so all the "close the
  others" lines go.
- Esc is `self.overlay = None` (or pop). Enter is `overlay.confirm()`. Arrow keys are
  `overlay.step()`.
- Make the menu actions gpui actions (`Box<dyn Action>`) or a single `enum AppAction`.
  Then `gd_ui::MenuItem` can carry the action directly and the app-side `MenuRow`, the 4
  `menu_entry` fns and the 4 `*MenuView` structs all disappear. Each feature keeps only
  its `*_menu_rows()` fn (the "props") and its `run_*_action` (the logic).
- Dialogs: the same idea. Merge's and branch's `key_capture` (`merge/merge_dialog.rs:135`,
  `branches/branch_dialog.rs:127`) and history's `enter_confirms` are three
  implementations of "Enter confirms / Esc cancels while an input is focused". Put that
  once into `gd_ui::Dialog`, e.g. `.on_confirm(..)` also capturing `Enter`.

### 1.2 `Workspace` is still the god object, so components aren't independent

**Where.** `workspace/state.rs:22`: about 40 fields, covering settings, settings modal,
accounts UI, popover (8 fields: `popover`, `popover_gen`, `popover_dismissed`,
`popover_selected`, `new_branch_base`, `new_branch_base_open`, `filter`, `name_input`,
`url_input`, `popover_scroll`), menu, dialog, history UI, stash UI, merge UI, branch UI,
diff view, diff select, 5 scroll handles, commit message, …
`RepoModel` (`repo/state.rs:77`) has about 45 fields plus a 47-line `empty()`
constructor that only writes defaults.

Effects you can measure:
- **87** `ws: Entity<Workspace>` fields in components. Almost every leaf component
  depends on the root.
- **54** `ws.update(cx, |this, cx| this.something(..))` callbacks. **27** "clone-tuple
  dances" like `let (repo_a, repo_b, repo_c, ws) = (self.repo.clone(), …)`
  (`stashes/stash_header.rs:83`).
- Every feature's handlers are `impl Workspace` blocks scattered across 15 folders. To
  know what `Workspace` can do you must grep the whole crate.
- Every `*Ui` struct carries a comment like "A plain struct (not an `Entity`) because
  Esc/Enter routing in workspace.rs reads `menu`/`dialog`" (`stashes/state.rs:64`,
  `merge/state.rs:75`, `branches/state.rs:50`, `history/state.rs:45`). The routing in
  §1.1 is the only thing that stops these from being real stores.

**Simplification (Vue: one store per feature).** Once §1.1 exists, turn each `*Ui` into
`Entity<…Store>`, created by the feature itself:
- `PopoverStore` (the 10 popover fields), `HistoryStore`, `StashStore`, `MergeStore`,
  `BranchStore`, `AccountsStore`, `SettingsStore`, `DiffViewStore`.
- Components take **their** store, e.g. `Entity<StashStore>` instead of
  `Entity<Workspace>`, and call `store.update(cx, |s, cx| s.open_dialog(..))`.
  `Workspace` keeps only layout and the list of stores.
- `RepoModel` should group its loose fields too: `ChangesState { status, excluded, partial,
  selected, diff: DiffDetail }`, `HistoryLog { commits, exhausted, loading, selected,
  detail: DiffDetail }`, `RefreshState { refreshing, pending, count, last_watch, … }`.
  Then derive `Default` on the groups and `empty()` shrinks to 5 lines.
- The ARCHITECTURE "known perf TODO" ("split `Workspace` into child entities so typing in
  the commit box doesn't re-render the diff") is solved by the same change.

### 1.3 The "file list + diff + syntax highlight" flow exists four times

**Model side.** These four places hold the same set of state: files, selected file, diff,
highlight, generation counter, load task, highlight task, highlight cache.
- changes: `RepoModel.{change_diff, change_hl, change_diff_gen, _change_diff_task, _change_hl_task}` (`repo/change_diff.rs`)
- commit: `RepoModel.{commit_files, selected_commit_file, commit_diff, commit_hl, commit_gen, _commit_task, _commit_hl_task, commit_shown}` (`repo/commit_diff.rs`)
- stash: `StashState.{files, selected_file, diff, hl, gen_, task, hl_task, cache}` (`stashes/state.rs`, `stashes/ops.rs:70-167`)
- PR: `PrPreviewState.{selected_file, diff, hl, gen_, task, hl_task, cache}` (`pr_preview/ops.rs:118-166`)

The code that drives them is also copied:
- `select_commit`/`select_commit_file` (`repo/commit_diff.rs`) and
  `select_stash`/`select_stash_file` (`stashes/ops.rs:70,114`) are line-for-line the same.
  `select_pr_file` is a third copy.
- `spawn_highlight` (`repo/diff_highlight.rs:15`), `set_stash_diff`
  (`stashes/ops.rs:144`) and `set_pr_diff` (`pr_preview/ops.rs:145`) are three
  highlight loaders. `spawn_highlight` only covers two targets because `ExpandTarget` is a
  2-variant enum.
- There are **three `HighlightCache`s** (`RepoModel.hl_cache`, `StashState.cache`,
  `PrPreviewState.cache`), so an LRU meant to be shared isn't.
- `repo/expand.rs` has a `match target { Commit => …, Changes => … }` with two identical
  arms that only differ in field names.

**View side.**
- `render_history_main` (`history/commit_detail.rs:16`), `render_stash_detail`
  (`stashes/stash_detail.rs:15`) and `render_pr_preview`
  (`pr_preview/pr_preview_pane.rs:15`) all build the same layout: header, 260px
  `nav_focus` file list, `vdivider`, `render_diff`.
- `CommitFileList`, `StashFileList`, `PrFileList` are three ~40-line components that
  only differ in id, data source and select fn.

**Simplification (Vue: a `useDiffDetail()` composable plus a generic `<FileDiffPane>`):**
```rust
pub struct DiffDetail {
    pub files: Vec<FileChange>, pub selected: Option<usize>,
    pub diff: Option<Arc<Diff>>, pub hl: Option<Arc<DiffHighlight>>,
    gen: u64, task: Option<Task<()>>, hl_task: Option<Task<()>>,
}
impl DiffDetail {
    fn load_file(&mut self, ix, loader: impl FnOnce(&Repository, &FileChange) -> Result<Diff>, cache, cx);
    fn set_diff(&mut self, diff, path, cache, cx);   // the one highlight spawn
}
```
- One shared `HighlightCache` on `RepoModel`.
- One `FileList { id, files, selected, on_select }` component and one
  `FileDiffPane { header: AnyElement, detail: &DiffDetail, pane: DiffPane }`.
- `ExpandTarget` and `DiffPane` merge (they're the same concept: which detail to act on).
- What disappears: 3 highlight spawners, 3 select-file fns, 2 select-container fns, 3
  file-list components, 3 render-detail methods, 2 caches. That's roughly 350 lines.

### 1.4 Three op runners plus about 20 hand-written async blocks

- `RepoModel::run` (`repo/run.rs:11`) runs off-thread, clears `remote_op` **and**
  `committing` whatever the op was, sets `error`, refreshes.
- `RepoModel::run_op` (`merge/ops.rs:59`) does the same plus a `busy` label and a
  one-at-a-time guard, and lives in the **merge** feature although history and branches
  use it.
- `RepoModel::hist_run` (`history/ops.rs:65`) is `run_op` plus `history.multi.clear()`.
- On top of these there are 24 `background_spawn` calls. Most are the 15-line incantation
  `cx.spawn(async move |this, cx| { let res = cx.background_spawn(…).await;
  this.update(cx, |this, cx| { if gen != … { return } match res { Ok(..) => …, Err(e) =>
  this.error = Some(e.to_string()) } cx.notify(); }).ok(); }).detach();`.

**Simplification.** Keep two helpers in `repo/run.rs`:
- `fn run(&mut self, op: Op { label: Option<&str>, .. }, work, on_ok, cx)`: write ops,
  with busy label, refresh and error.
- `fn load<T>(&mut self, slot: impl Fn(&mut Self) -> &mut LoadSlot, work, apply, cx)`:
  generation-checked reads (`LoadSlot { gen, task }`).

That removes the generation-counter boilerplate from `change_diff.rs`, `commit_diff.rs`,
`history_log.rs`, `stashes/ops.rs`, `pr_preview/ops.rs`, `history/ops.rs`
(`reload_push_state`), `accounts/handlers.rs` (`load_identity`) and `merge/handlers.rs`
(`merge_select`).

### 1.5 Component ceremony: 74 `RenderOnce` structs, 42 of them with a do-nothing `new()`

The convention "default to a `RenderOnce` struct, one per file" was applied even where a
component is only a struct with a `new()` that copies its arguments into fields, followed
by a `render` that forwards them. Examples:
- `diff/unified_list.rs` and `diff/split_list.rs`: 11- and 10-field structs whose
  `render` destructures `self` and calls `rows_list(…)`. They could be two plain fns or one
  `DiffList`.
- `diff/diff_options_button.rs` (27 lines for one `icon_button`), `diff/diff_pane.rs`
  (15 lines, an enum), `diff/whitespace_note.rs` (`new(bool) -> Option<Self>`),
  `changes/sidebar_tabs.rs`, `changes/changes_list.rs`, `history/multi_select.rs`
  (`new(repo)` only).
- `accounts/accounts_page.rs:22` `AccountsView` is a struct that copies four fields of
  `AccountsUi` so the page can own them. Pass `&AccountsUi` (or the store entity) instead.
- `settings/diff_page.rs:9` `DiffOptionValues` re-declares three `Settings` fields.

This doesn't contradict "one template per file": the template should still be a file.
The problem is the **parameter-copying layer** (`new` + struct + destructure) that sits
between the caller and the template. With per-feature stores (§1.2), most components can
read their store directly and need one prop instead of 5–11. When a struct is still
needed, make the props `pub` and build it with a struct literal; the 42 trivial `new()`s
then go away. CONVENTIONS.md should say when a plain `fn x(..) -> impl IntoElement` is
enough (no state, one call site).

### 1.6 Same widget, several copies: make them generic and override locally

| Concept | Copies | Make one |
|---|---|---|
| Branch row (icon + name + trailing) | `pickers/branch_picker.rs:101` `branch_row`, `merge/merge_candidate_list.rs:59` `row`, `pickers/new_branch_base_list.rs:60` `base_row`, `history/cherry_pick_branch_row.rs`, inline in `pr_preview/pr_base_menu.rs:45` | `shared/branch_row.rs` with `.trailing(el)`, `.selected()`, `.on_click()`, `.on_right_click()` |
| File row | `changes/change_row.rs` (`file_name`:49, `status_badge`:88) and `shared/file_change_row.rs` | one `FileRow` with an optional leading checkbox slot |
| File list | `CommitFileList`, `StashFileList`, `PrFileList` | `FileList` (§1.3) |
| Resize handle + `NoGhost` | `changes/sidebar_handle.rs` and `history/message_handle.rs` (both define `struct NoGhost`) | `gd_ui::ResizeHandle::horizontal/vertical(payload).on_reset(..)` |
| Multi-line input styling | `changes/commit_box.rs:35` and `history/history_dialog.rs:150` copy the same 15 builder calls | `shared/text_field.rs` → add `text_area_field` next to `text_field` |
| Full-window popover (backdrop + positioned card + `menu_in` + `handoff`) | `ui/overlay.rs` ContextMenu, `diff/diff_options_menu.rs`, `pickers/popover_layer.rs`, `pr_preview/pr_base_menu.rs`, `settings/settings_modal.rs` (re-implements `Dialog`'s scrim) | `gd_ui::Popover::anchored(pos).width(w).on_dismiss(..)` + `Dialog::custom(card)` |
| Diff display options UI | `diff/diff_options_menu.rs:78-97` and `settings/diff_page.rs:26-52` (same Tabs + two `CheckOptionRow`s) | one `DiffDisplayOptions` component used in both |
| "Branch list minus remotes that have a local twin" | `pickers/filter.rs:27` `filtered_branches` and `merge/handlers.rs:18` `merge_candidates` | one fn on `RepoModel` |
| Line look (bg, fg, marker) | `diff/unified_row.rs:39` `line_look` and the `match` in `diff/split_row.rs:103` | reuse `line_look` |
| "Mixed" dash mark | `ui/components.rs` `Checkbox` Mixed and `diff/row_select.rs` `handle_mark` | `gd_ui::mixed_mark()` |
| Commit message join | `changes/message.rs:13` `join`, `history/confirm.rs:18` `SquashMessage::full`, plus 3× in gd-git | one `gd_git::message(summary, desc)` |
| Reveal / copy path actions | `menus/menu_actions.rs:51` `reveal` and `merge/conflict_handlers.rs:72` (different fallback: parent vs root) | one `shared/file_actions.rs` |
| `CheckOptionRow` | lives in `features/diff/` but is used by settings | move to `shared/` |

### 1.7 Over-splitting and confusing file names

- `features/diff/` has **27 files** for one viewer. Ten of them are under 45 lines
  (`diff_pane`, `diff_options_button`, `gutter_style`, `scroll_x`, `whitespace_note`,
  `rows_list`, `text_flow`, `unified_list`, `split_list`, `mod`).
- Near-identical names that a newcomer can't tell apart:
  `text_select.rs` / `text_selection.rs` / `text_handlers.rs`,
  `row_select.rs` / `line_selection.rs`,
  `expander.rs` / `expand_button.rs` / `repo/expand.rs`,
  `message.rs` (changes) / `message_resize.rs` / `message_handle.rs` (history).
- Suggested regrouping, Vue-style with subfolders per sub-feature:
  ```
  features/diff/
    diff_view.rs            (render_diff entry + DiffFrame)
    list/ unified.rs split.rs rows.rs split_model.rs
    gutter/ number_cell.rs block_handle.rs line_selection_store.rs
    text_selection/ model.rs mouse.rs handlers.rs
    expand/ buttons.rs
    options/ options_menu.rs
    highlight.rs  store.rs
  ```
  This keeps one template per file but makes the folder tree explain itself.

---

## 2. `gd-git` crate

| Finding | Where | Fix |
|---|---|---|
| "Reset index → `add -A` in chunks of 500 → `stage_partial` → build message → `commit -F -` → `rev-parse HEAD`" written 3× | `lib.rs:127` `commit_partial`, `history_ops.rs:190` `amend_commit`, `merge.rs:118` `commit_merge` | private `fn stage(paths, partial)`, `fn message(summary, desc)`, `fn head_sha()` |
| "Old side = Blob(rev^1, old_path) unless New; new side = Blob(rev, path) unless Deleted" written 4× | `diff.rs:102` `commit_diff`, `stash.rs:241` `stash_diff`, `pr_preview.rs:80` `compare_file_diff`, partly `working_diff` | `fn blob_sides(old_rev, new_rev, file) -> (Side, Side)`; each caller becomes 1 line |
| `parse_name_status` duplicated | `stash.rs:264` and the inline copy in `log.rs:96` `commit_files`; `pr_preview` imports it from **stash** | move to its own `name_status.rs`, use it in all 3 |
| `rev-list --count` helper ×2 | `merge.rs:27` `count`, closure in `pr_preview.rs:65` | one `pub(crate) fn count` |
| 3 branch-creation fns | `lib.rs:224` `create_branch`, `history_ops.rs:143` `create_branch_at`, `merge.rs:175` `create_branch_from` | `create_branch(name, base: Option<&str>, track: bool)` |
| `.run()?; Ok(())` tail on ~25 fns, `res.map(\|_\| ())` before every `outcome_of` | everywhere | `Git::exec(self) -> Result<()>`; `outcome_of(Result<String>)` |
| Branch-management fns live in `merge.rs` | `merge.rs:162-178` (rename, delete remote, create-from) | move to `refs.rs` (the branch file) |
| Remote URL parsing lives in `history_ops.rs` | `history_ops.rs:373` `split_remote`, `:394` `github_web_url`; `pr_preview.rs:96` `host_of` repeats the "contains github" test | new `remote.rs`: `split_remote`, `host_of`, `web_url`; `github_web_url` = `host_of == GitHub` |
| Hand-rolled percent-encoding ×2 | `pr_preview.rs:127` `enc`, app `shared/file_url.rs:4` | `percent-encoding` / `url` crate (`Url::from_file_path`), as CONVENTIONS asks |
| `.git` file (`gitdir:`) parsing ×2 | `lib.rs:349` `git_dir`, `exec.rs:37` `portal_layout` | `portal_layout` calls a shared `resolve_git_dir(root)` |
| `pub type CommitFile = FileChange;` | `log.rs:48` | delete the alias, use `FileChange` |
| `eprintln!` in a library | `lib.rs:247` | `log::warn!` |
| Test-only public API | `lib.rs:32` `credential_env_for_tests` (used only by an example) | goes away once examples become tests (below) |
| Examples used as tests | `examples/{smoke,history_ops,merge_smoke,stash_smoke,identity_smoke}.rs` are `fn main` with `assert!`, run by hand | move to `crates/git/tests/*.rs` with a `tempdir` helper so `cargo test` runs them; keep only `diff_compare` as an example |
| `"WIP on (no branch)"` format ×2 | `stash.rs:38` `Stash::title`, `stash.rs:184` `stash_rename` | one `fn subject(branch, msg)` |
| `DiffLine` hunk-header / no-newline / context constructors repeated, tab expansion ×2 | `engine.rs:124,214`, `expand.rs:56,110,144` | `DiffLine::hunk(..)`, `::no_newline()`, `::context(..)` |
| `expand.rs` private `Hunk` re-declares `DiffHunk` + rows | `expand.rs:48` | `struct Hunk { meta: DiffHunk, rows }` |
| Stale doc | `diff.rs:66`: "`None` for … parsed-from-git diffs" (there is no git diff parsing any more) | fix |
| Clippy | collapsible `if`s at `lib.rs:351`, `exec.rs:158`; complex type `engine.rs:275` (`(Vec<(usize,usize)>, Vec<…>)`) | `type Emphasis = Vec<Range<usize>>` |

---

## 3. `gd-ui` crate

| Finding | Where | Fix |
|---|---|---|
| Dead tokens | `theme.rs:80` `BUBBLE_RADIUS`, `:86` `TEXT_SM`, `:87` `TEXT_BASE`: **0 uses**, while the app hard-codes `text_size(px(12.0))` 44×, `px(13.0)` 16×, `px(12.5)` 8×, `px(11.5)` 7× | use the tokens everywhere (`TEXT_XS=11.5`, `TEXT_SM=12`, `TEXT_MD=12.5`, `TEXT_BASE=13`, `TEXT_LG=15`) or add `.text_sm()` style helpers |
| Dead fn | `theme.rs:183` `Theme::for_mode`: 0 callers (replaced by `themes.rs` `for_family`) | delete |
| Alias | `theme.rs:356` `hairline(a)` is just `ink(a)`, 1 caller | delete |
| Hidden global state | `theme.rs:337` static `LIGHT` flag read by `ink()`, `is_light()`, `gutter_style::handle_bg`, `Button` hover | put `ink(a)` on `Theme` (`theme.ink(a)`). Every caller already has `cx` or `theme`. Then `set_light`, `preview` vs `for_family` and the "doesn't touch the global" comments all go away |
| `Theme::dark()`/`light()` are a hand-written 5th and 6th palette next to the data-driven `themes.rs` palettes | `theme.rs:90-180` | express zeron as two more `Palette` consts, so `from_palette` is the only constructor |
| Unused builder | `overlay.rs:263` `Dialog::hide_cancel` | delete |
| Tabs badge duplicates `badge()` | `components.rs:380` | use `badge()` |
| `Button` Primary hover hack: `hsla(0,0,if is_light {0} else {1},1)` | `components.rs:198` | a `solid_hover` theme token |
| Stale comments | `motion.rs:11` links to `docs/PERFORMANCE.md` (doesn't exist); `theme.rs:267` mentions `history::graph_color` (doesn't exist) | fix |
| gd-ui depends on gd-syntax only for `HighlightKind` | `Cargo.toml`, `theme.rs:231` `SyntaxPalette::color` | fine, but `SyntaxPalette` has 30 fields of which most map to 7 colors. Store the 7 base colors and map in `color()`, which deletes ~60 lines of `build` / `from_palette` field lists |

## 4. `gd-syntax` crate (ported from zeron, carries unused generality)

The app calls exactly `highlight(HighlightRequest { source, path, fence_tag: None })` and
`language_for_path`. These are therefore unused:
- `fence_tag`, `detect_language`'s fence branch, `highlight_with_limits` (cancellation is
  always `None`), `HighlightLimits` + `Default` + `DEFAULT_MAX_*` (the app has its own
  `MAX_BYTES`/`MAX_LINES` in `diff/highlight.rs:25`, the same 2 MB),
  `HighlightedDocument::language`, `supports_language`, `GrammarUnavailable`.
- Markdown fenced-code injection of 16 languages (`injected_languages`, the
  `markdown_inline` config) is a zeron chat feature. Diffs of `.md` files don't need
  highlighted code fences.
- `configuration()` is the 2nd-longest function in the repo (153 lines). It could be a
  table of `(LanguageId, language_fn, highlights_query, injections)` rows.

Trimming this to "path → language → spans" would cut roughly 300 of its 839 lines.
Keep it if you plan to re-sync with zeron; otherwise trim.

---

## 5. App crate: per-feature findings

### 5.1 `repo/`
- `run.rs` resets `remote_op` and `committing` after **every** op (`run.rs:22`), while
  `run_op` resets `committing` too. Ops shouldn't clear flags they didn't set. A single
  runner (§1.4) owns this.
- `this.commits.clear(); this.selected_commit = None;` ×5 (`merge/ops.rs:101` has a
  private `reset_history_selection` that the other 4 copies in branches, stashes and
  changes can't see). Make it a `pub(crate)` method on `RepoModel`.
- `status.as_ref().and_then(|s| s.branch.clone())` ×6, plus `stashes/ops.rs:14`
  `current_branch()` duplicating `branch_name()` minus the placeholder;
  `"(detached HEAD)"` literal ×2. Add `RepoModel::current_branch() -> Option<&str>`.
- `selected_change.and_then(|ix| self.changes().get(ix))` ×4. Add `selected_change_file()`.
- `RefreshScope::from_metadata_changed` and `or` are a hand-written 2-variant bool. A
  `bool full` would do, or keep the enum but drop the helpers.

### 5.2 `workspace/`
- `"Workspace Menu"` literal (`render.rs:178`) next to the `MENU`/`WORKSPACE` consts in
  `actions.rs`. Build it from the consts.
- `if let Some(repo) = &self.repo { repo.update(cx, …) }` ×10, plus
  `let Some(repo) = self.repo.clone() else { return }` ×16, plus `stashes/handlers.rs:118`
  `stash_op` that wraps the same thing. Add one `fn with_repo(&self, cx, f)`.
- `navigation.rs:45` `select_move` is a 70-line `match` with 6 arms of "compute target,
  call `select_*`, scroll". With `DiffDetail` (§1.3) and a `trait NavigableList`, most arms
  collapse.

### 5.3 `features/changes`
- Two functions named `default_summary`: `RepoModel::default_summary` (`ops.rs:20`) and the
  free fn `state::default_summary(r, amending)` (`state.rs:44`) wrapping it. The
  "merging" check `merge.in_progress….commit_finishes()` is repeated 3×. Add
  `RepoModel::merging()` and keep one `default_summary`.
- The placeholder is computed twice: `CommitButton.default_summary` and
  `Workspace.message_placeholder` + `sync_message_placeholder` (`message.rs:30`, a gpui
  layout-cache workaround). Document the workaround in one place and derive both from one
  call.
- Inline plurals despite `shared/plural.rs`: `changes_header.rs:46`, `ops.rs:80`,
  `stashes/ops.rs:179`, `shared/relative_time.rs:40`.
- `no_changes.rs:70` opens `format!("file://{root}")` without encoding, while
  `shared/file_url.rs` exists for exactly that. **Paths with spaces break.**
- `sidebar.rs:15` `SidebarBody` enum: clippy flags a 504-byte variant size difference.
  Box it, or render directly with `match`.

### 5.4 `features/history`
- `state.rs:59` `AmendState { root: PathBuf }`: a one-field wrapper. Use `Option<PathBuf>`.
- `message_handle.rs:68` `pub(super) use MessageDrag as Drag;`: a rename alias with no
  reason. Delete it.
- `confirm.rs:12-21` `SquashMessage` + `full()` duplicates `message::join`.
- `ops.rs:8` doc points to `Repository::history_op_in_progress` (doesn't exist; it's
  `in_progress`).
- It has its own menu system (§1.1).

### 5.5 `features/merge` and `features/branches`
- `merge/ops.rs:182` `message_event(summary, description) -> RepoEvent` just builds an enum
  variant (1 caller). Delete it and write `RepoEvent::PreparedMessage { .. }` inline.
- `run_op` (the generic runner) lives here (§1.4).
- `MergeUi::wants_menu_context` / `BranchUi::wants_menu_context` are one-liners that only
  exist for §1.1.
- `branches/handlers.rs:80` `snapshot_menu_action(which: usize)`: a test hook with a magic
  number in production code. Expose `BranchAction` under `cfg(snapshot)` instead.
- `RemoteBranch::parse` (`branches/state.rs:43`), `pr_head_name`, `pr_base_on_remote`,
  `Repository::checkout`, `remote_button.rs:41` and `merge_candidates` each do their own
  `split_once('/')` on `remote/branch`. Add one `fn split_remote_branch(&str)` in gd-git.

### 5.6 `features/stashes`
- `ops.rs:169` `after_stash_op()` = `self.stashes.viewing = false`, 2 callers. Inline it.
- `handlers.rs:118` `stash_op`: see §5.2.
- Menus: stash actions are variants of the generic `MenuAction`, and `run_menu_action`
  first offers every action to `run_stash_action` (`menus/menu_actions.rs:17`). That makes
  `menus` and `stashes` depend on each other. With §1.1 the stash menu owns its own
  action enum like history does.

### 5.7 `features/pickers`
- `step_picker` (`handlers.rs:85`) and `step_popover_selection`
  (`popover_handlers.rs:84`) are **identical**. Delete one.
- 10 popover fields on `Workspace` (§1.2). Move them to `PopoverStore`.
- `popover_dismissed` (the "backdrop click must not reopen the trigger" workaround,
  `popover_handlers.rs:10`) is clever but undocumented outside a comment. Keep it, and
  move it into the generic `Popover` (§1.6) so it's solved once.

### 5.8 `features/pr_preview`
- `state.rs:19` `loaded_base` is **written, never read**. Delete it.
- `ops.rs:169` `pr_file_diff` is a one-call wrapper. Inline it.
- The rest is covered by §1.3.

### 5.9 `features/settings` and `features/accounts`: hand-rolled persistence
- 4 files in 3 formats: `repositories` (`*`-prefixed lines), `display` (`key=value`),
  `accounts` (TSV), `tokens` (TSV, 0600).
  Code that exists only for that: `Settings::{apply_display, display_text,
  apply_repositories, repositories_text}`, `ThemeMode` `Display`/`FromStr`,
  `accounts/model.rs` `{accounts_text, tokens_text, parse, clean}`, `Provider::{key,
  from_key}`. That's about 150 lines plus their tests.
- CONVENTIONS.md itself says "prefer a crate for serialization". Use `serde` + `toml`
  with one `settings.toml` (plus `tokens.toml` at 0600, keeping `write_private`). Old-file
  compatibility is the only argument for the current format, and this is a 0.1 POC.
- `accounts/handlers.rs:213` `persist_accounts(&mut self, _cx)` takes an unused `cx`.
- `StatusLine` (state) and `StatusText` (component) are two types for "message + is
  error". `StatusText::new(&StatusLine)` just copies it.
- `accounts/model.rs:104` `new_id()` hand-rolls a unique id. `uuid` would do, or keep it;
  it's small.

### 5.10 `features/notices`, `toolbar`, `welcome`
- Mostly fine. `toolbar/toolbar.rs` inside `toolbar/` triggers clippy's
  `module_inception`; rename it `toolbar_bar.rs` or move the struct to `mod.rs`.
- `notice_banner.rs:80` hashes the message to make an animation key. Fine, but the same
  `DefaultHasher` boilerplate is also in `diff/highlight.rs`. Add a `shared/hash.rs`
  one-liner.

### 5.11 `platform/`
- `platform/mod.rs:5` documents `config_base_dir()`, which no longer exists (it moved to
  `dirs`). ARCHITECTURE.md lines 541-542 have the same sentence spliced into the middle of
  another one.
- `linux/chrome.rs:203` `frame`: 8 almost identical `edges.push(strip(…)…)` lines and a
  `let px_ = |v| px(v)` alias. Use a table of `(ResizeEdge, top, left, right, bottom, w,
  h, cursor)`.

### 5.12 `shared/`
- `plural.rs` (5 lines), `split_path.rs` (7), `file_url.rs` (13), `branch_name.rs` (16):
  one file per 5-line fn. With your "shared logic in a utils file" preference, merge the
  string helpers into `shared/text.rs` (plural, split_path, sanitize_branch_name,
  relative_time) and the path helpers into `shared/paths.rs` (file_url, reveal,
  copy_path). That's 2 named util files instead of 6 micro-files, which fits both your
  preference and the "name it after what it does" rule.
- `relative_time.rs`: a crate (`timeago` / `chrono-humanize`) per CONVENTIONS, or keep it.
  It's small and tested.

---

## 6. Dead, write-only or pointless code (safe deletions)

| Item | Where | Evidence |
|---|---|---|
| `PrPreviewState::loaded_base` | `pr_preview/state.rs:19` | only assigned (`ops.rs:107`) |
| `Theme::for_mode` | `ui/src/theme.rs:183` | 0 callers |
| `BUBBLE_RADIUS`, `TEXT_SM`, `TEXT_BASE` | `ui/src/theme.rs:80-87` | 0 uses |
| `hairline()` | `ui/src/theme.rs:356` | alias of `ink()` |
| `Dialog::hide_cancel` | `ui/src/overlay.rs:263` | 0 callers |
| `CommitFile` type alias | `git/src/log.rs:48` | alias of `FileChange` |
| `credential_env_for_tests` | `git/src/lib.rs:32` | test-only public API |
| `message_event` | `app/.../merge/ops.rs:182` | builds an enum variant |
| `after_stash_op` | `app/.../stashes/ops.rs:169` | one assignment |
| `stash_op` | `app/.../stashes/handlers.rs:118` | `if let Some(repo)` wrapper |
| `pr_file_diff` | `app/.../pr_preview/ops.rs:169` | one-call wrapper |
| `use MessageDrag as Drag` | `app/.../history/message_handle.rs:68` | rename alias |
| `step_popover_selection` **or** `step_picker` | `pickers/` | identical bodies |
| `AccountsView` | `accounts/accounts_page.rs:22` | field copy of `AccountsUi` |
| `DiffOptionValues` | `settings/diff_page.rs:9` | field copy of `Settings` |
| `AmendState` | `history/state.rs:59` | one-field wrapper |
| `px_` closure | `platform/linux/chrome.rs:543` | alias of `px` |
| `_cx` param of `persist_accounts` | `accounts/handlers.rs:213` | unused |
| `MergeUi::wants_menu_context` | `merge/state.rs:114` | `self.menu.is_some()` |

---

## 7. Tests and the snapshot harness

- `snapshot.rs` is 697 lines. Its `render` closure is **457 lines** (the longest function
  in the repo), a `match view { "split" | "split-history" | … }` over about 60 string
  names, plus `history_phase2` and `accounts_sample`. It needs
  `#[cfg(feature = "snapshot")]` re-exports sprinkled through feature `mod.rs` files
  (`diff/mod.rs:41`, `history/mod.rs:29`, `accounts/mod.rs:19-22`) and test hooks such as
  `snapshot_menu_action`.
  - Fix: a `static VIEWS: &[(&str, fn(&mut Workspace, &mut Window, &mut Context<Workspace>))]`
    table. Each feature exposes `#[cfg(feature="snapshot")] pub fn snapshot_views() ->
    &[...]` next to its own code, Vue-storybook style. `snapshot.rs` shrinks to the
    harness (window, capture, sleep, PNG).
- `watch_check` and `process_cpu_ticks` in `snapshot.rs` are an integration test of the
  watcher, hidden in a PNG tool. They belong in `tests/`.
- gd-git's real behaviour tests are `examples/*` run by hand (§2).

---

## 8. Documentation and comment drift (misleads humans the most)

| Says | Reality | Where |
|---|---|---|
| `stash_file_row` component | doesn't exist (`FileChangeRow` is shared) | `docs/CONVENTIONS.md:80` |
| `Repository::history_op_in_progress` | it's `in_progress` | `history/ops.rs:8` |
| `RepoModel::start_highlight` | it's `spawn_highlight` | `diff/highlight.rs:2` |
| `docs/PERFORMANCE.md` | doesn't exist | `ui/src/motion.rs:11` |
| `config_base_dir()` | replaced by `dirs` in settings | `platform/mod.rs:5`, ARCHITECTURE.md:541 |
| "routing in workspace.rs / menus.rs" | `workspace/actions.rs`, `menus/key_routing.rs` | `stashes/state.rs:64`, `merge/state.rs:75`, `branches/state.rs:50`, `history/state.rs:45` |
| "see repo.rs" | `repo/` folder | `stashes/ops.rs:2` |
| `history::graph_color` | doesn't exist | `ui/src/theme.rs:267` |
| "Known perf TODOs: … add syntax highlighting with a cache; add a file watcher" | both done | ARCHITECTURE.md §Performance |
| integration branch `rust-rewrite` | current work is on `main` / feature branches | AGENTS.md, PROGRESS.md |
| "Stale results are dropped … (`change_diff_gen`, A status refresh always reloads…" | a sentence got spliced into another one | ARCHITECTURE.md:337 |
| "parsed-from-git diffs" | git diff parsing was removed | `git/src/diff.rs:66` |

ARCHITECTURE.md (414 lines) is also more of a changelog of gotchas than a map. For
humans, split it into `ARCHITECTURE.md` (a 1-page map: crates, stores, data flow, threading)
and `GOTCHAS.md` (the GPUI-CE list, which is genuinely valuable). The TODO item "debug
mode that shows unique strings to find the code for an element" becomes much less
necessary once components read their own store: the element id prefix then equals the
store and file name.

---

## 9. Suggested order of work

Each step leaves the app working and makes the next one easier.

1. **Deletions and doc fixes** (§6, §8, clippy lints). Low risk, one sitting.
2. **gd-git dedup** (§2) and moving examples to `tests/`. It's isolated and has tests to
   guard it.
3. **Generic components** (§1.6): `BranchRow`, `FileRow`, `FileList`, `ResizeHandle`,
   `text_area_field`, `Popover`, `DiffDisplayOptions`.
4. **One op runner + `load` helper** (§1.4).
5. **`DiffDetail` store** (§1.3). This removes the largest amount of copied logic.
6. **Overlay store + one menu system** (§1.1). Biggest readability win; it touches every
   feature.
7. **Per-feature stores** (§1.2). Mechanical after step 6: each `*Ui` becomes
   `Entity<*Store>`, and components switch `Entity<Workspace>` → their store.
8. **serde settings** (§5.9), `snapshot.rs` table (§7), `diff/` folder regrouping (§1.7),
   `gd-syntax` trim (§4).

After step 7, update `docs/CONVENTIONS.md` to describe the store-per-feature model, the
"generic component + local override" rule, and when a plain fn beats a `RenderOnce`
struct, so future sessions (human or AI) don't re-grow the copies.

---

# Part 2: make it lighter (libraries instead of hand-written code)

Goal: **keep every feature, but own as little code as possible.** The rule used below:
replace our code with a library only when that library is widely used and maintained
(used by cargo, ripgrep, bat, Zed, gitui and similar). No obscure one-person crates.
Every estimate is approximate, and the trade-off is stated when there is one.

Today: ~23k lines of Rust (gd-git ~3.3k + 0.7k examples, gd-syntax 0.8k, gd-ui 1.9k,
app ~16.6k). Part 1 + Part 2 together should land around **11–13k lines**, and more
importantly far fewer *kinds* of code a reader must understand.

## P2.1 Read the repository with `git2` and keep the `git` CLI only for writes and network

CONVENTIONS.md says "git **writes and network ops** keep shelling out to the `git` CLI"
(for hooks, credential helpers, LFS, SSH). Reads were never part of that rule. Today every
read is a `git` process plus a hand-written parser of its text output.
[`git2`](https://docs.rs/git2) (libgit2 bindings maintained by the rust-lang org, used by
cargo and gitui) returns typed data instead.

| Ours today (hand-written) | git2 replacement | Deleted |
|---|---|---|
| `status.rs` porcelain-v2 parser (`parse_status`, `status_from_xy`, 86-line fn) | `repo.statuses()` + `Status` flags, `graph_ahead_behind` | most of `status.rs` |
| `refs.rs` `for-each-ref` parser | `repo.branches(None)`, `branch.upstream()` | most of `refs.rs` |
| `log.rs` `%x1f` log parser, paging | `repo.revwalk()` + `skip/take` | the parser |
| `parse_co_authors`, app `body_without_trailers` | `git2::message_trailers_strs` | both |
| `parse_name_status` ×2, `commit_files`, `stash_files`, `compare` | `diff_tree_to_tree(..).deltas()` | 3 parsers |
| `stash.rs` list parser + `parse_subject` + `stash_index` re-resolution | `repo.stash_foreach` (index + message + oid) | ~120 lines |
| `progress.rs` `in_progress()`: reads `rebase-merge/`, `rebase-apply/`, `msgnum`, `end`, `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `SQUASH_MSG` by hand | `repo.state()` (`RepositoryState::{Merge, Rebase*, CherryPick, Revert, …}`) + `repo.open_rebase()` for step/total | ~120 lines |
| `has_unmerged`, `merge_preview` conflict list (`merge-tree --write-tree` parsing) | `repo.index()?.has_conflicts()`, `repo.merge_commits(..)` → `index.conflicts()` | both |
| `git_dir()`, `common_git_dir()` (`.git` file / `commondir` parsing), `last_fetch_time` path | `repo.path()`, `repo.commondir()` | ~40 lines |
| `remotes()`, `default_remote`, `tracking_remote_url`, `unpushed_commits`, `tags_by_commit`, `describe_commit`, `merge-base`, `rev-list --count` | `repo.remotes()`, `branch_upstream_remote`, revwalk with `hide`, `tag_foreach`, `merge_base`, `graph_ahead_behind` | ~150 lines |
| `identity.rs` reads (`git config --get` with exit-1 handling) | `repo.config()?.get_string("user.name")`, `Config::open_default()` | read half |
| `read_side` (`cat-file blob` + "does not exist" stderr matching) | `repo.revparse_single("rev:path")?.peel_to_blob()` | stderr string matching |

**Edge cases that disappear with it:** stderr text matching (`"does not exist"`,
`"Not a valid object"`, `"does not have"`, `"CONFLICT"`), `LC_ALL=C` /
`core.quotepath=false` / `color.ui=false` output pinning (needed only for parsing), the
`trim_start_matches('\n')` record fixes, the `%x1f`/`%00` separators, and
`allow_exit_one`.

**What stays CLI:** `commit`, `add`, `reset`, `checkout`, `merge`, `rebase`, `revert`,
`cherry-pick`, `stash push/pop/apply/drop/store`, `push`, `pull`, `fetch`, `clone`,
`config` writes, `hash-object`/`update-index` for partial staging. That's a thin
`Git::new(..).args(..).run()` per op, and `exec.rs` shrinks with the reads gone (keep
`portal_layout` for Flatpak, see P2.9).

**Trade-off:** libgit2 is a C dependency (build it with the `vendored-libgit2` feature so
users need nothing installed), and its status can differ from the CLI in exotic setups
(fsmonitor, sparse checkout, `core.untrackedCache`). gitui ships exactly this split.
Also: two engines means a read right after a CLI write must reopen or refresh the
`git2::Repository` (it caches the index). Use one `Repository::open` per background task.

**Estimate:** gd-git ~3.3k → ~1.5k lines.

Also in git land:
- `git add -- <paths>` in **chunks of 500** (ARG_MAX workaround, ×3): use
  `git add --pathspec-from-file=- --pathspec-file-nul` with the paths on stdin. One call,
  no chunking, no `--literal-pathspecs` juggling for weird file names.
- Remote URL → host/owner/repo (`split_remote`, `host_of`, `github_web_url`,
  `clone_dir_name`): use [`gix-url`](https://docs.rs/gix-url), part of gitoxide, which
  parses scp-style, ssh:// and https:// exactly like git.
- Building PR/compare and `file://` URLs (`enc`, `file_url`): the
  [`url`](https://docs.rs/url) crate (`Url::from_file_path`, `query_pairs_mut`).

## P2.2 Drop the `gd-syntax` crate and use `syntect`

`gd-syntax` (839 lines + 16 tree-sitter grammar crates, ported from zeron) plus
`SyntaxPalette` (30 fields ×3 constructors) plus the per-theme syntax colors in `themes.rs`
can be replaced by [`syntect`](https://docs.rs/syntect) (used by bat, delta, zola), with
its built-in grammars for ~all languages instead of our 17.
- Even better for themes: Nord, Catppuccin, Solarized, Tokyo Night and VS Code all exist as
  `.tmTheme` files, so syntax colors come from the theme file instead of our hand-mapped
  `HighlightKind` → palette table (`SyntaxPalette::{dark, light, build, from_palette, color}`
  and the `Palette.{comment, keyword, string, …}` fields all go away).
- Keep our side-reconstruction in `diff/highlight.rs` (old side = context+deletes, new side
  = context+adds); syntect highlights line by line, which fits it even better.
- **Trade-off:** syntect is slower than tree-sitter (ARCHITECTURE says 18k rows in ~125 ms
  with tree-sitter). Diffs are capped at 20k rows and highlighting already runs in the
  background, so this is acceptable. The original reason ("share queries with zeron") no
  longer matters once you stop syncing with zeron.
- **Deleted:** `crates/syntax/` entirely, ~150 lines of palette code, 16 grammar
  dependencies (faster cold builds).

## P2.3 Theme and colors: use what GPUI-CE already ships

All verified in `.ref/gpui-ce` at the pinned rev:
- `gpui::rgb(0xRRGGBB)` exists → delete `themes.rs` `hex()`.
- GPUI already depends on [`palette`](https://docs.rs/palette) (OKLCH, mixing) → delete the
  hand-written `oklch_to_srgb` matrix math and `mix()` in `theme.rs`/`themes.rs`, or simply
  write zeron's colors as hex like every other theme and drop OKLCH altogether.
- `gpui::ease_out_quint()` / `ease_in_out` exist → delete `motion.rs` `CubicBezier` (Newton +
  bisection solver). Zeron's `cubic-bezier(0.16,1,0.3,1)` is visually an ease-out-quint/expo.
- Zeron as two `Palette` consts like the other themes → `Theme::dark()`/`light()` (90 lines
  each) and the special "zeron has no palette" `Option` in `ColorTheme` go away.
- Remove the global `LIGHT` atomic (P1 §3): `theme.ink(a)` instead of a free `ink(a)`.
- **Estimate:** `theme.rs` + `themes.rs` + `motion.rs` ~830 → ~350 lines.

## P2.4 Assets: `rust-embed`

`assets.rs` keeps an `icons!` macro that lists every icon three times (enum, path,
`include_bytes!`) so that `AssetSource::list("")` can enumerate them (a documented GPUI-CE
gotcha). [`rust-embed`](https://docs.rs/rust-embed) embeds the `assets/` folder and gives
`Assets::iter()` for `list()` and `Assets::get()` for `load()`. Adding an icon becomes
"drop the SVG in the folder". Keep `Icon` as a small enum of names if you want
autocomplete. Fonts are loaded the same way.

## P2.5 File watcher: `notify-debouncer-full` + `ignore`

`watcher.rs` (251 lines) hand-builds a thread, an mpsc protocol (`Msg::{Event, Stop}`),
trailing debounce with a max wait, and a hard-coded skip list (`.git`, `target`,
`node_modules`).
- [`notify-debouncer-full`](https://docs.rs/notify-debouncer-full) (official companion of
  `notify`, which we already use) does the debounce, the thread and stop-on-drop.
- [`ignore`](https://docs.rs/ignore) (ripgrep's gitignore matcher) replaces the hard-coded
  skip list with the repo's real `.gitignore`. That's more correct (a Python `venv/` or a
  `dist/` folder is skipped too) and less code.
- Keep: the "reads don't count" event-kind filter and `classify_git`. Those are the
  genuinely subtle bits.
- **Estimate:** 251 → ~90 lines.

## P2.6 Settings, accounts, tokens

- `serde` + `toml` for settings and account metadata (P1 §5.9). `dirs` stays for the path.
- Tokens: [`keyring`](https://docs.rs/keyring) (Secret Service / macOS Keychain / Windows
  Credential Manager, the standard choice). `accounts/model.rs` explicitly rejected it
  because "D-Bus fails on headless boxes". **That is an edge case that never happens for a
  desktop GUI app**, which always has a session bus. Deleted: the `tokens` file format,
  `write_private` (0600 handling with `cfg(unix)`/`cfg(not(unix))`), `clean()`,
  `tokens_text`. It is also more secure than a plaintext file.
- `new_id()` → [`uuid`](https://docs.rs/uuid) v4.
- **Estimate:** settings + accounts model ~420 → ~120 lines.

## P2.7 Small hand-rolled helpers → crates

| Ours | Library |
|---|---|
| `shared/relative_time.rs` (+ tests) and `last_fetched.rs` | [`timeago`](https://docs.rs/timeago) or [`jiff`](https://docs.rs/jiff)'s `Span` formatting |
| `HighlightCache` (hand-written LRU over a `Vec` behind a `Mutex`) | [`lru`](https://docs.rs/lru) or [`quick_cache`](https://docs.rs/quick_cache) |
| `engine.rs` word emphasis: `tokenize`, `word_emphasis`, `emphasize_pairs` (~110 lines) | `similar`'s own `TextDiff::iter_inline_changes` (enable the `inline` feature of the crate we already use) |
| `engine.rs` `split_lines` | `similar`'s line tokenization (`TextDiff::from_lines`) |
| `percent-encode` ×2 | `url` / `percent-encoding` |
| `notice_banner.rs`/`highlight.rs` `DefaultHasher` boilerplate | `std::hash::BuildHasher::hash_one` (std, one line) |

## P2.8 Diff view: let GPUI do the horizontal scrolling (needs a spike)

The manual horizontal scroll is a big chunk of custom machinery for one feature:
`DiffView.hscroll`, `HExtent`, `set_split_extent`/`set_unified_extent`, `scroll_x.rs`
(shapes the longest line with a hand-built `TextRun` to clamp), `Diff::longest_line`
(computed in the engine *and* recomputed in `expand`), `restrict_scroll_to_axis`, the
`on_scroll_wheel` x-delta handler, and `TextFlow::Scroll(x)` shifting every cell.
GPUI-CE's `uniform_list` has `with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)`
(verified in `.ref/gpui-ce`), which is how Zed scrolls wide lists horizontally.
- **Trade-off:** with native scrolling the line-number gutters scroll away with the text
  unless they're rendered as a separate fixed column. The split view would need its two
  halves as two lists sharing one vertical scroll handle. Try it on the unified view first.
  If gutters-stay-fixed is a must, keep today's approach.
- **If it works:** ~200 lines and the `longest_line` field go away.

## P2.9 Edge cases we handle that (almost) never happen

Each line is code that a reader has to understand for a case most users never hit. My
recommendation is in the last column. "Drop" means accepting the rare failure, which then
shows up as a normal git error in the banner.

| Edge case | Where | Recommendation |
|---|---|---|
| Path lists longer than ARG_MAX (500-path chunks) | `lib.rs`, `history_ops.rs`, `merge.rs` | replace with `--pathspec-from-file` (P2.1), one code path |
| Non-UTF-8 file contents: lossy detection to disable partial staging | `diff.rs:166` | keep (4 lines, protects user data) |
| Symlink targets / submodule dirs in `read_side` | `diff.rs:141` | gone with git2 blob reads |
| Linked worktrees: `.git` *file*, `commondir` | `lib.rs:349`, `exec.rs:41`, `watcher.rs` | gone with git2 (`repo.path()`, `commondir()`) |
| Flatpak document-portal cwd (`portal_layout`) | `exec.rs:37` | **keep**: you ship a flatpakref, so this is real. git2 reads don't need it; only CLI writes do |
| Pre-existing `GIT_CONFIG_COUNT` in the env when adding credential helpers | `credentials.rs:46` | drop (only matters if the user exports `GIT_CONFIG_COUNT` themselves) |
| Headless box without D-Bus for token storage | `accounts/model.rs` | drop → `keyring` (P2.6) |
| Unborn HEAD (no commits yet) special paths in `commit_partial` / `discard` / `working_diff` | `lib.rs` | keep for commit (first commit is common); `discard` before the first commit can just be `git rm --cached` + delete |
| `stash rename` via `store` + `drop` index arithmetic | `stash.rs:183` | keep (feature), but with `stash_foreach` from git2 the index re-resolution is trivial |
| Squash-merge paused state (`SQUASH_MSG` + unmerged index) as its own `InProgress::Squash` variant | `progress.rs` | merge into a generic "index has conflicts" state: git2's `repo.state()` reports Clean here, so show the conflict banner whenever `index.has_conflicts()` |
| Rebase `rebase-merge` vs `rebase-apply` step files | `progress.rs:81` | gone with `open_rebase()` |
| `merge-tree --write-tree` unavailable on old git (conflicts `None`) | `merge.rs:40` | gone with git2 `merge_commits` |
| Highlight runs that split a UTF-8 char or overrun the text (`runs_for` guards) | `diff/highlight.rs:81` | keep (prevents a GPUI panic), but syntect returns line-relative ranges, so most guards go |
| Exact CSS cubic-bezier solver | `motion.rs` | drop → `ease_out_quint` (P2.3) |
| Watching new top-level dirs created at runtime; symlink cycles | `watcher.rs` | handled by `notify` recursive watch + `ignore` |
| `RefreshScope` "pending refresh while one is in flight" coalescing | `repo/refresh.rs` | keep (it fixed real refresh loops), but it becomes a 10-line struct once the watcher debounces properly |
| Backdrop click re-opening the popover trigger (`popover_dismissed`) | `pickers/popover_handlers.rs` | keep, but move into the one generic `Popover` component |
| gpui-ce layout cache showing a stale placeholder (`sync_message_placeholder`) | `changes/message.rs:30` | keep (framework bug), add a link to an upstream issue so it can be removed later |

## P2.10 The common components folder

Rule: **a component lives in a feature folder only if exactly one feature uses it.** Used by
two or more features → `components/`. Pure visual primitives with no app types (no
`RepoModel`, no `gd_git`) → the `gd-ui` crate. Non-UI helpers → `utils/`.

```
crates/
  ui/ (gd-ui)                 primitives, no app knowledge
    button.rs checkbox.rs tabs.rs badge.rs icon.rs tooltip.rs divider.rs
    context_menu.rs dialog.rs popover.rs        ← one overlay family (P1 §1.1, §1.6)
    resize_handle.rs text_field.rs text_area.rs option_row.rs check_row.rs
    theme/ (palette.rs, themes/*.tmTheme or palettes)  motion.rs  assets.rs
  app/src/
    components/                app-aware generic components (props + local overrides)
      branch_row.rs            ← 5 copies today (P1 §1.6)
      file_row.rs              ← change_row + file_change_row
      file_list.rs             ← commit / stash / PR file lists
      file_diff_pane.rs        ← header + file list + diff (P1 §1.3)
      diff/                    ← the diff viewer itself is a reusable component
      suggestion_card.rs field_row.rs status_text.rs empty_state.rs
      diff_display_options.rs  ← options popover + settings page
    stores/ (or each feature's store.rs)        one Entity per feature (P1 §1.2)
    utils/  text.rs (plural, split_path, sanitize_branch_name)  paths.rs (file_url, reveal, copy_path)  async_ops.rs (run/load, P1 §1.4)
    features/<feature>/        only that feature's templates + store + ops
```

How "generic with local overrides" looks in Rust/GPUI (the Vue props/slots equivalent):
```rust
// components/branch_row.rs: one template
BranchRow::new(ix, &branch)
    .selected(ix == sel)
    .trailing(relative_time(branch.tip_time))   // slot
    .on_click(move |w, cx| ...)                 // emit
    .on_right_click(move |pos, w, cx| ...)      // optional
// merge dialog, branch picker, new-branch base list, cherry-pick dialog and
// PR base menu all use this, each with its own trailing slot / handlers.
```
Builder methods are the props, `impl IntoElement` arguments are the slots, and callbacks
are the emits. One file, one template, overrides at the call site.

## P2.11 The biggest possible offload: a component library (needs a trial first)

[`gpui-component`](https://github.com/longbridge/gpui-component) (Longbridge's GPUI
component library, written by Jason Lee and featured on
[Zed's blog](https://zed.dev/blog/community-champion-jason-lee)) ships Button, Checkbox,
Tabs, Input/TextArea, Popover, ContextMenu with keyboard navigation, Dialog/Modal,
Notification, Resizable panels, Tooltip, virtualized List/Table and a theme system. That
covers most of `gd-ui` (~1.9k lines), the menu/overlay keyboard routing (P1 §1.1), resize
handles, notices and picker keyboard handling.

**Unverified, and it's the deciding factor:** gpui-component targets Zed's upstream
`gpui`, and this project is pinned to the **GPUI-CE** fork. I couldn't confirm that the
two are compatible. Adopting it may mean moving from GPUI-CE to upstream gpui and
re-checking every gotcha in ARCHITECTURE.md (including the vendored parley patch). It
would also mean re-skinning its components to the zeron look in DESIGN.md through its
theme system.
**Recommendation:** a one-day trial on a branch (render one dialog and one context menu
with gpui-component on the pinned gpui-ce rev). If it compiles without forking, it's the
single largest reduction available. If not, P2.10's own `gd-ui` overlay family is the
fallback.

## P2.12 Revised order of work

1. P1 §6 deletions + §8 doc fixes (no risk).
2. **P2.11 trial** (one day). Its result decides whether `gd-ui` work happens at all.
3. P2.3 theme/motion/assets via GPUI built-ins + `rust-embed` (small, isolated).
4. **P2.1 git2 for reads** + `--pathspec-from-file` + `gix-url`/`url`, and move examples
   to `tests/` first so the change is guarded.
5. P2.5 watcher, P2.6 settings/keyring, P2.7 small crates.
6. P2.2 syntect (drops a crate).
7. P2.10 `components/` + P1 §1.6 generic components.
8. P1 §1.4 async helpers → §1.3 `DiffDetail` → §1.1 overlay store → §1.2 per-feature stores.
9. P2.8 native horizontal scroll trial; `snapshot.rs` table (P1 §7).
10. Rewrite CONVENTIONS.md: libraries-first rule, `components/` rule, store per feature,
    and replace the "no `utils.rs`" rule with "`utils/` files named by topic".

---

# Part 3: adopt `gpui-component` (gpui-ce fork), tested 2026-10-25 (NOT ADOPTED)

> Kept for reference only: we decided to keep our own components.

[gpui-ce/gpui-component](https://github.com/gpui-ce/gpui-component) is the GPUI-CE fork of
Longbridge's component library (`main` @ `c082069`, 2026-10-01). A reference clone is in
`.ref/gpui-component`, and the trial crate is in `/tmp/desktop-rs/gpc-trial/`. This
**replaces P2.11** and is the single largest simplification available.

## P3.1 What it replaces

| Ours (hand-built) | gpui-component |
|---|---|
| Overlay stack, z-order, Esc chain (`workspace/render.rs` `overlays()`, `workspace/actions.rs`) | `Root` window layers: `window.open_dialog(..)`, `open_sheet_at`, `push_notification(..)` |
| 5 menu systems, `MenuRow`/`RowItem`, 4 `*MenuView`, 5 step loops (P1 §1.1) | `menu::{ContextMenu, PopupMenu, DropdownMenu}` (keyboard navigation built in) |
| `gd_ui::Dialog`, merge/branch `key_capture`, history `enter_confirms` | `Dialog` / `AlertDialog` (`on_ok`, `on_cancel`, Esc via `.keyboard(true)`) |
| Pickers (filter input + `uniform_list` + arrows + Enter, `PickerShell`, `popover_selected`, `step_picker`…) | `List` + `ListDelegate` / `searchable_list` |
| `features/notices` + auto-dismiss task | `Notification::success()/error()` |
| Sidebar resize + commit message resize handles, both `NoGhost`s | `h_resizable` / `v_resizable` |
| `platform/linux/chrome.rs` (CSD frame, caption buttons, resize strips) | `TitleBar` + `window_border()` (honors compositor-supported buttons) |
| `crates/syntax` (tree-sitter, 17 languages) | `highlighter` (tree-sitter, 30+ languages, feature-gated). This is better than P2.2 (syntect), because it keeps tree-sitter's speed |
| `theme.rs` + `themes.rs` (~660 lines of Rust palettes, OKLCH math, `ink()` global) | JSON themes + `ThemeRegistry` (ships Catppuccin, Solarized, Tokyo Night, Gruvbox, Ayu, …) |
| `gd_ui` Button, Checkbox, Tabs, badge, tooltip, `OptionRow`, `text_field`, commit text area | `Button`, `Checkbox`, `Radio`, `Switch`, `Tab`, `Badge`, `Tooltip`, `Input`/`TextArea` (`auto_grow(min, max)`, soft wrap) |
| `motion.rs` (curves, `handoff`) | theme `MotionTokens` (easing + springs) |
| Settings modal, nav, pages | `setting` module (pages, groups, fields) |

The files that overlap total about **5.4k lines**, plus the 0.8k-line `gd-syntax` crate.
Realistically **~4–5k lines go away**, together with the most intricate code we own
(overlay routing, CSD frame, theme math). `gd-ui` shrinks to a few app-specific styled
helpers, or disappears into `app/src/components/` (P2.10 still applies to *our* generic
components: `BranchRow`, `FileRow`, `FileList`, `FileDiffPane`, the diff viewer).

## P3.2 The blocker: it doesn't build on our GPUI-CE pin (yet)

The trial depended on gpui-component and used `[patch.crates-io]` to point it at our pinned
`gpui-ce` rev `9a82fba` (with our `vendor/gpui_ce_parley` patch).
- ✅ Cargo unified everything into **one** `gpui-ce`.
- ❌ `gpui_ce_components_base` failed with **164 errors**.

Why: gpui-component targets **crates.io `gpui-ce` 0.2.2**. Our git pin also says 0.2.2,
but it is newer, after the text system moved to **parley**. The errors:
- ~40 text-layout API errors (`ShapedLine`, `LineWrapper`, `x_for_index`,
  `wrap_boundaries`, `LineFragment`). Its input/text editor uses the pre-parley API.
- ~120 smaller drifts: 73 "multiple applicable items" (trait-method ambiguity with new
  GPUI traits), 20 changed argument counts, 17× `App::notify` gone, missing struct fields,
  new `TextAlign::{Start, End}` variants.

Checked against the crates.io 0.2.2 source, here is what we would lose by moving back to
it:
- **parley**: the vendored `OverflowWrap::Anywhere` patch (long words wrapping in the
  commit box). gpui-component's `TextArea` has its own wrapping; whether it breaks a
  single very long word is **unverified**.
- **`.transitions(..)`** (animated hover washes): not in 0.2.2. gpui-component's
  components animate through their own `MotionTokens`, and we'd stop hand-styling hovers
  anyway.
- `hover_style`/`active_style`: renamed back to `.hover()`/`.active()`. Mechanical.
- Everything else we rely on **is in 0.2.2**: `client_inset`, `on_mouse_up_all`,
  `restrict_scroll_to_axis`, `button_layout`, `window_controls`, `ease_out_quint`,
  `ListHorizontalSizingBehavior`, `StyledText::with_runs`, `index_for_position`.

## P3.3 Options (your call)

| Option | Effort | Owned code afterwards | Risk |
|---|---|---|---|
| **A. Move to crates.io `gpui-ce` 0.2.2 + gpui-component** (drop the git pin and `vendor/gpui_ce_parley`) | Medium: it's a UI rewrite on top of library components anyway | Lowest; no forked or vendored GPUI code | Long-word wrapping in the commit box to re-check; lose CE commits newer than 0.2.2 |
| B. Keep our pin and fix gpui-component for parley ourselves | High: 164 errors, ~40 of them in its text editor | We'd own a fork of a large library | Contrary to the goal |
| C. Wait: ask gpui-ce/gpui-component to support the parley-era gpui-ce, meanwhile do Part 2 (git2, watcher, serde, keyring, …), which is UI-independent | Low now | Unchanged UI code until upstream moves | Unknown timeline |

**Recommendation: A, with C's issue filed in parallel.** Moving onto the published
versions both crates are designed for means owning *no* GPUI-level code. Today we carry a
vendored text crate and a git pin. Most of our UI code would be replaced by gpui-component
anyway, so the APIs lost in the downgrade mostly disappear with the code that used them.
Do it on a branch:
1. Switch `gpui`/`gpui_platform`/`gpui_elements` to the crates.io versions gpui-component's
   `Cargo.toml` uses and remove `[patch]` + `vendor/`.
2. Add gpui-component, wrap the window in its `Root`, set up `TitleBar`, theme registry,
   and a zeron JSON theme (dark + light) next to the bundled themes. Nord and VS Code need
   JSON files too (small, color-only).
3. Rebuild overlays (dialogs, menus, notifications, pickers) on gpui-component. This
   *is* P1 §1.1 and §1.2, now mostly by deletion.
4. Check the commit box with a very long single word. If it doesn't wrap, either accept
   it (rare edge case: P2.9 spirit) or file it upstream.
5. Then proceed with Part 2 (git2 for reads, etc.) and the remaining Part 1 items.

## P3.4 Updated overall order

1. P1 §6 deletions and §8 doc fixes.
2. **Part 3 option A** (branch): gpui-component on crates.io gpui-ce. This deletes the
   overlay/menu/picker/dialog/notification/theme/CSD/syntax code instead of refactoring it.
3. P2.1 git2 for reads + `--pathspec-from-file` + `gix-url`/`url` (move gd-git examples to
   `tests/` first).
4. P2.5 watcher, P2.6 settings/keyring, P2.7 small crates, P2.4 rust-embed (only for
   what gpui-component's asset crate doesn't already cover).
5. P1 §1.3 `DiffDetail` + P2.10 `components/` (BranchRow, FileRow, FileList, FileDiffPane).
6. P1 §1.2 per-feature stores (much smaller now that overlays live in `Root`), P1 §1.4
   async helpers.
7. P2.8 native horizontal scroll trial, `snapshot.rs` table, CONVENTIONS.md rewrite.

---

# Part 4: component audit and cleanup plan (2026-10-26)

We keep our own components. This is the inventory of all **74 components** (`impl
RenderOnce`/`Render`) plus the hot-row fns, with a verdict for each. Rule (from P2.10):
- **gd-ui** = primitives with no app types (no `RepoModel`, `Workspace`, `gd_git`), one per file.
- **`app/src/components/`** = generic app components used by 2+ features.
- **`features/<x>/`** = components only that feature uses.

Every step is verified against **79 deterministic snapshots** (fixtures with fixed commit
dates: `/tmp/desktop-rs/tools/{fixtures,snap,cmp}.sh`). A step is accepted only if they
are pixel-identical, or if every differing image is an intended change that was looked at.

## P4.1 gd-ui (primitives)

| Component | Verdict |
|---|---|
| `components.rs` (486 lines: `Button`, `Checkbox`, `Tabs`, `ToolbarButton` + `pressable`, `icon`, `divider`, `vdivider`, `badge`, `overlay_card`) | **Split, one file per component** (`button.rs`, `checkbox.rs`, `tabs.rs`, `toolbar_button.rs`, `surface.rs` for the small style fns). `Tabs` badge reuses `badge()`. |
| `overlay.rs` (`ContextMenu`, `Dialog`) | **Split** into `context_menu.rs`, `dialog.rs`; delete unused `Dialog::hide_cancel` |
| `tooltip.rs` | keep |
| `theme.rs` dead items | delete `for_mode`, `hairline`, `BUBBLE_RADIUS`, `TEXT_SM`, `TEXT_BASE` |
| **new** `ResizeHandle` | replaces `changes/sidebar_handle.rs` + `history/message_handle.rs` (two copies, two `NoGhost`) |
| **new** `OptionRow` (radio card) | moved from `stashes/option_row.rs` (generic, no app types) |
| **new** `CheckRow` | `diff/check_option_row.rs` made generic (callback instead of `Entity<Workspace>`), used by diff menu + settings |
| **new** `Popover` (backdrop + anchored card + `menu_in` + `handoff`) | replaces the hand-made layers in `diff_options_menu.rs` and `pickers/popover_layer.rs` |
| **new** `EmptyState` (centered faint text) | replaces ~8 copies ("No local changes", "No commits yet", "No matches", "No matching branches", PR empty states, …) |

## P4.2 `app/src/components/` (new folder, app-aware generic components)

| Component | Replaces |
|---|---|
| `FileRow` | `changes/change_row.rs` row body + `shared/file_change_row.rs` (same dim-dir / bright-name / status badge; changes adds a checkbox and conflict pill via builder options) |
| `FileList` | `history/commit_file_list.rs`, `stashes/stash_file_list.rs`, `pr_preview/pr_file_list.rs` |
| `BranchRow` | `pickers/branch_picker.rs` `branch_row`, `merge/merge_candidate_list.rs` `row`, `pickers/new_branch_base_list.rs` `base_row`, `history/cherry_pick_branch_row.rs`, inline row in `pr_preview/pr_base_menu.rs` |
| `MenuView` (renders any `Vec<MenuRow<A>>` + one `step()`) | `menus/context_menu_view.rs`, `history/commit_menu.rs`, `branches/branch_menu.rs`, `merge/conflict_menu.rs` (4 copies of `menu_entry`) + 4 copies of the arrow-key loop |
| `SuggestionCard`, `text_field`, **new** `text_area` | moved from `shared/`; `text_area` replaces the copies in `changes/commit_box.rs` and `history/history_dialog.rs` |
| `DiffDisplayOptions` | the Unified/Split tabs + two check rows duplicated in `diff/diff_options_menu.rs` and `settings/diff_page.rs` |
| `MenuRow` | moved from `shared/menu_row.rs` |

`shared/` keeps only non-UI helpers and is renamed `utils/` (`text.rs`: plural,
split_path, sanitize_branch_name, relative_time, commit_authors; `paths.rs`: file_url,
status colors move to `components/file_row.rs`).

## P4.3 Feature components: keep, with these fixes

- **Delete pass-through structs:** `accounts/accounts_page.rs` `AccountsView` (copy of
  `AccountsUi` fields), `settings/diff_page.rs` `DiffOptionValues` (copy of `Settings`
  fields), `history/message_handle.rs` `use MessageDrag as Drag` alias.
- **Fix the copy-paste id:** `branches/branch_menu.rs` uses `("merge-menu", gen)`, the same
  id as the conflict menu.
- Everything else (headers, panels, dialogs, banners, pages, toolbar, welcome) is used by
  one feature and is fine where it is. Its remaining issues (props copied out of
  `Workspace`, the `Entity<Workspace>` coupling) are the store refactor (P1 §1.2), not a
  component problem.

## P4.4 Steps

1. gd-ui: split files, delete dead code (no visual change).
2. gd-ui: add `ResizeHandle`, `OptionRow`, `CheckRow`, `EmptyState`, `Popover`; migrate users.
3. `components/`: `FileRow`, `FileList`, `BranchRow`, `MenuView` + generic step,
   `text_area`, `DiffDisplayOptions`; move `shared/` UI pieces; `shared/` → `utils/`.
4. P4.3 fixes; update CONVENTIONS.md / ARCHITECTURE.md component sections.
