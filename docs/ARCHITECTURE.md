# Architecture

## Workspace layout

```
.
├── Cargo.toml          workspace; GPUI-CE pinned by git rev
├── crates/
│   ├── git/   (gd-git)  UI-agnostic git layer, shells out to `git`
│   ├── forge/ (gd-forge) blocking CI API client for GitHub/GitLab/Gitea/Forgejo + job log parser, no GPUI
│   ├── syntax/ (gd-syntax) UI-agnostic tree-sitter highlighting
│   ├── ui/    (gd-ui)   zeron-style design system: theme, motion, components, assets
│   └── app/   (desktop-rs)  the binary: models + views
├── docs/
└── .ref/ target/   (git-ignored; see below)
```

`.ref/` holds read-only reference clones: `.ref/zeron` (the design source) and
`.ref/gpui-ce` (the framework source at our pinned rev). Read those instead of
guessing APIs. To recreate them:
`git clone --depth 1 https://github.com/zeronsh/zeron .ref/zeron` and the same for `gpui-ce/gpui-ce`.

## Git layer (`crates/git`)

- **Everything goes through the `git` CLI** (`exec.rs`), reads and writes alike. No libgit2/`git2`.
  Like Desktop's `dugite`, this gives the user's exact git: hooks, credential helpers, config, LFS and SSH
  behave as on the command line, with no libgit2 feature gaps.
- `exec.rs` pins `LC_ALL=C`, `GIT_OPTIONAL_LOCKS=0`, `-c core.quotepath=false` and `-c color.ui=false`.
  Network ops set `GIT_TERMINAL_PROMPT=0`, so credentials must come from a helper or ssh-agent.
  (An auth UI is still to do.)
- Reads parse machine-readable output only: `status --porcelain=v2 -z`, `log` with `%x1f`/NUL separators,
  `for-each-ref` with `%00`, `show --name-status -z`, `--shortstat`, `rev-list --count`,
  `merge-tree --write-tree`, `cat-file blob` / `show rev:path`, `ls-files -u -z`, `git config`.
  Never scrape human-readable output.
- `Repository::open` runs one `rev-parse --path-format=absolute --show-toplevel --git-dir --git-common-dir`
  and caches the root and git dirs. Each call spawns a fresh `git`, so nothing is cached between calls.
- **All functions block.** Call them only from a background executor.
- Commit model (`Repository::commit`): reset the index to HEAD, `add -A` the
  checked paths, then `commit -F -`. This matches Desktop's checkbox semantics.
  Partial files go through `commit_partial` / `amend_commit(.., partial)`, see §Partial staging.
- Diffs are parsed into a flat `Vec<DiffLine>` (hunk headers included) so the UI
  can index row `i` in O(1) from a `uniform_list`. Parsing is capped at 50k lines (`truncated`).
- `crates/git/tests/*.rs` exercise the write paths against scratch repos (`tests/common` has the
  tempdir + `sh` helpers): `cargo test -p gd-git`. Index staging goes through
  `git add --pathspec-from-file=- --pathspec-file-nul` (no ARG_MAX chunking). Remote URL parsing lives in `remote.rs`.

## Forge crate (`crates/forge`, `gd-forge`)

Pure blocking Rust (`ureq` 2, the major `oauth2` 5's `ureq` feature uses, so one copy is linked), no GPUI, no async.
Call it from background threads. It never does auth: the caller passes platform, host and token
(`Client::new(Forge, host, token)`, sent as `Authorization: Bearer`). The platform comes from the account settings and fixes
the capabilities (`Client::capabilities`); there is no probing. API base: github.com -> `https://api.github.com`, GitHub
Enterprise -> `https://<host>/api/v3`, GitLab `/api/v4`, Gitea and Forgejo `/api/v1`.

- Files: `client.rs` (`Forge`, `Client`, HTTP, error mapping, dispatch), `model.rs` (GitLab vocabulary: `Status`, `Pipeline`,
  `Stage`, `Job`, `Step`, ...), `actions.rs` (GitHub and Gitea share the Actions REST shape), `forgejo.rs`, `gitlab.rs`,
  `workflow.rs` (stages from a workflow file), `repo_ref.rs` (`RepoRef::from_remote` reuses `gd_git::split_remote`; GitLab id = encoded full path),
  `error.rs` (`Unauthorized`, `Forbidden`, `NotFound`, `RateLimited`, `Unsupported`, `Http`, `Network`, `Parse`; a GitHub 403 with
  `x-ratelimit-remaining: 0` or `retry-after` is `RateLimited`). Fixtures of trimmed API responses: `src/tests/fixtures/`.
- Pages are 1-based, 20 per page, newest first. List entries have no commit title (GitLab: pipeline name) and no GitLab user;
  `pipeline()` fills both (one extra commit request on GitLab).
- Stages: GitLab has real ones (jobs sorted by id, stages by first appearance, retried jobs dropped, bridges included). GitHub and Gitea
  jobs have none: `pipeline()` returns one stage named after the workflow, and the UI calls `stages_from_workflow(yaml, jobs)` with the file
  from the working tree (`Pipeline::workflow_path`) to get "Stage N" columns by `needs` depth (matrix names `job (a, b)` and
  reusable `job / inner` match by prefix; `continue-on-error: true` sets `allow_failure`). Forgejo's job list carries `needs`, so its stages
  come from the API (`workflow_path` is only the file name there).
- `retry_pipeline` means "retry failed jobs" everywhere (GitLab retry, GitHub/Gitea `rerun-failed-jobs`).
- `PipelineDetail::passed_with_warnings()`: pipeline succeeded but an `allow_failure` job failed.

| | GitHub | GitLab | Gitea | Forgejo |
|---|---|---|---|---|
| pipelines / pipeline / job_log | yes | yes | yes | yes |
| real stages | no (workflow file) | yes | no (workflow file) | yes (`needs`) |
| steps | yes | no | yes | no |
| retry pipeline (failed jobs) | yes | yes | yes | no |
| retry job | yes | yes | yes (run id in URL) | no |
| cancel pipeline | yes | yes | yes | yes |
| cancel job | no | yes | no | no |
| play manual job | no | yes | no | no |

Logs (`log.rs`): `parse_log(raw) -> Log { lines, sections }`. A `Line` is a `String` plus `Span`s (byte ranges covering the whole text, with
`Style { fg, bg: Option<Color::{Indexed, Rgb}>, bold, italic, underline }`) and the GitHub/Gitea timestamp if there was one. SGR is parsed with `vte`; other control
sequences are dropped; `\r` keeps the last segment; SGR state carries across lines. Sections come from GitLab `section_start`/`section_end` (nested,
`collapsed=true`, duration from the timestamps) and GitHub `##[group]`/`##[endgroup]` (collapsed by default); `Section { header, end, depth, .. }`
index into `lines` (body = `header + 1 .. end`), so a virtualized list can hide collapsed ranges without copying. Unclosed sections end at the last line.

Assumed minimum versions (swagger checked against gitea.com 29.0-dev and codeberg.org Forgejo 16-dev; older servers may lack
endpoints and answer `NotFound`): GitHub REST 2022-11-28 (Enterprise Server 3.x), GitLab 15+, Gitea 1.25 (runs, jobs, logs, rerun;
`status`, `limit`), Forgejo 12+ (actions runs and jobs API).

## Stashes (`gd-git/src/stash.rs`, `app/src/features/stashes/`)

- Listing: `git stash list -z --format=%H%x1f%h%x1f%ct%x1f%cd%x1f%gs` with `--date=format-local`. `%gs` is
  `WIP on <branch>: <sha> <subject>` (no user message) or `On <branch>: <msg>`; `parse_subject` extracts branch/message
  (`(no branch)` = detached). Stashes are identified by sha, never by index: write ops re-resolve `stash@{n}` from the sha
  right before running (indices shift when others are dropped).
- Layout: tracked changes = `git diff S^1 S`; untracked files (`-u`) live in the parentless third parent `S^3`
  (`ls-tree -r` for the list, `git show S^3 -- path` for the diff). Untracked entries get `FileStatus::Untracked`.
- Push uses `-u` always; pathspec pushes pass `--literal-pathspecs` (paths like `notes [1].md`). Gotcha: the global
  `--literal-pathspecs` breaks `-u` cleanup when no pathspec is given, so it is only passed with paths.
- `exec.rs` now falls back to stdout for the error text when stderr is empty (`stash apply` prints CONFLICT lines on stdout).
- Rename = `stash store -m <new subject> <sha>` then drop the old entry (moves to top).
- State: `RepoModel::stashes: StashState` (`features/stashes/state.rs`, ops in `ops.rs`; list, filter, collapsed, selection, `detail: DiffDetail`);
  loaded in the background from every full refresh. `RepoEvent::SwitchBranchWithChanges` makes `RepoModel::checkout`
  hand dirty-tree checkouts to the Workspace, which owns `Workspace::stash_ui` (dialog + message input). Use
  `RepoModel::checkout_now` to skip the dialog. `gd_ui::Dialog::content` embeds inputs/option rows.
- `select_change` clears `stashes.viewing`, which is what swaps the main pane between stash detail and the working diff.

## Diff engine (`crates/git/src/engine.rs`, the `similar` crate)

- git only _fetches_ the two versions (`Side::{Empty, Blob{rev,path}, Worktree(path)}` via
  `Repository::read_side`: `cat-file blob rev:path`, or the filesystem). The diff is computed
  in-process (`engine::diff_texts`: Myers via `capture_diff_slices_deadline` with an 800ms deadline,
  `group_diff_ops(…, 3)` for hunks). Working, commit and stash diffs all go through
  `Repository::diff_sides`.
- Hide whitespace = compare lines with all whitespace removed (git `-w` semantics). A missing
  final newline is part of the line key, so newline-only changes show git's "\ No newline" row.
- Word emphasis: in each replace block the i-th deleted row pairs with the i-th added row.
  Lines are tokenized into word/space/punct tokens and diffed again, and changed byte ranges go to
  `DiffLine::emphasis`. It's skipped above 1000 chars or when more than 60% of both lines changed. It's
  rendered as a background wash by `features/diff/highlight.rs` `line_text` (merged with the syntax runs).
- `Diff::source` keeps the exact old/new texts (for partial staging). It's `None` when either side
  isn't valid UTF-8, so staging never writes back lossily decoded bytes. Binary = NUL in the first 8000
  bytes, like git. `too_large` above 8 MB per side.
- Line counts: per file they come from `Diff::added/deleted`, shown by `components/DiffStat` above the diff in the commit,
  stash and PR panes (never in the Changes pane). Totals per commit / PR are `Repository::commit_stats` and
  `Comparison::stats` (`git diff --shortstat -M`, parsed in `diff_summary.rs`), stored in `DiffDetail::stats`.
- Cross-check vs git: `cargo run --release -p gd-git --example diff_compare -- <repo> [N]`.
  On this repo's last 150 commits, 468/472 files had identical +/- counts. The rest differ where git's
  Myers heuristics are non-minimal (e.g. yarn.lock, where ours is smaller).

## Partial staging (`gd-git/src/partial.rs`, `app/src/features/diff/gutter/`: `line_selection_store.rs`, `number_cell.rs`, `block_handle.rs`)

- No `git apply`: `staged_text(diff, include)` replays the diff over `DiffSource::old` (raw lines via
  `split_inclusive`, so CRLF and a missing final newline survive) keeping only included adds/deletes.
  `stage_partial` checks the worktree still equals `source.new` (else errors: "changed on disk"), then
  `hash-object -w --stdin --path=<p>` + `update-index --add --cacheinfo <mode>,<sha>,<p>` (mode = worktree exec bit
  on unix, else HEAD's). Renames also `--force-remove` the old path.
- App state: `RepoModel.work.partial: HashMap<path, LineSelection { diff: Arc<Diff>, excluded: HashSet<row> }>` next to
  `excluded` (whole file off). `set_rows_included` normalizes: all rows off → `excluded`, none off → entry removed, so
  `partial` only holds mixed files. A reloaded diff keeps the selection only if `lines` are identical
  (`reconcile_selection`); `-w` diffs are ignored there. `partial` is cleared after a successful commit/amend.
- UI: `render_diff(.., select: Option<Entity<RepoModel>>, ..)`. `gutter/number_cell.rs` has `number_cell` (one cell
  holding 1 or 2 numbers; plain, or clickable/drag-extending with the blue+check included style) and
  `RowSelect::handle_cell` in `gutter/block_handle.rs` (one cell of a change block's handle), shared by the unified list and the split list's
  middle gutter. Handles work on change blocks (`RepoModel::change_block`: the add/delete run around a row), not git
  hunks. Pointer state (`hovered_handle: (block start, cell)`, `drag` anchor) is `Workspace.diff_select`, reset when
  the diff key changes. The cell id in `hovered_handle` makes leave/enter order between two cells of one block
  harmless. Drag ends via `on_mouse_up_all` on the list wrapper and only extends while `pressed_button == Left`.
- Hover states can be captured headlessly: `GD_SNAPSHOT_MOUSE=x,y` (window px) makes `--snapshot` dispatch a
  MouseMove there before capturing (`hover_style` and `on_hover` both react).
- Colors: number cells `gutter_bg()` (ink 2.5%), idle handle strip `handle_bg()` (black 60%, visibly darker),
  unselected block handle ink 10%; included line cells `line_selected_bg()` (blue-700, hover `accent_strong`),
  included/partial blocks `accent_strong` (hover `accent`).
- Gotcha: `on_mouse_up_all` takes 5 args (event, phase, hitbox, window, cx).

## Hunk expansion (`gd-git/src/expand.rs`, `features/diff/expand/buttons.rs`)

- `DiffHunk` carries 0-based `old_start/old_len/new_start/new_len`. `expand(diff, Up(h)|Down(h)|All(h))` is pure: it
  splits the flat rows into hunks, pulls context rows from `source.new` into the gap (20 = `EXPANSION_STEP`), merges
  touching hunks, re-flattens with fresh headers. Changed rows are copied untouched, so `staged_text` gives the same
  result before and after expanding (tested). `gap_above`/`gap_below` drive which buttons show.
- `RepoModel::expand_diff(pane, e)` swaps `detail(pane).diff` and re-highlights in the background.
  `change_diff_key` (path + options) lets `load_change_diff` keep the expanded diff when a reload returns the same
  source texts. Line selection is keyed by `LineKey = (is_add, line number)` instead of row index, so it survives
  row shifts; `reconcile_selection` drops it only when the source texts change.
- `render_diff(.., pane: DiffPane, ..)` (`diff/view.rs`): `Changes` (select + expand), `Commit` (expand), `Stash` and `Pr` (view only). `DiffPane` also names the `DiffDetail` (`RepoModel::detail(pane)`).
  Header rows render `Expander::gutter` instead of number cells; an extra trailing list row (index == rows) holds
  the last Expand Down. Buttons use `group` + `group_hover_style` so the svg icon recolors on hover.

## Threading / state (`crates/app`)

- `Workspace` (`workspace/`: `state.rs` struct, `init.rs`, `render.rs` root template, `actions.rs` key routing + Esc order, `repos.rs` open/add/clone, `repo_commands.rs`, `navigation.rs`, `overlay.rs` overlay stack) is the root view. It owns settings, picker state,
  text-input entities, scroll handles, and an `Option<Entity<RepoModel>>`.
- `RepoModel` (`repo/`: `state.rs`, `refresh.rs` (RefreshScope, watcher, `RefreshState` in `RepoModel::sync`), `background_fetch.rs`, `diff_detail.rs` (`DiffDetail` + the one highlight spawner, `select_file`, `expand_diff`), `change_diff.rs`/`commit_diff.rs`, `history_log.rs`, `run.rs`) owns all repository state, grouped like the features: `work: ChangesState` (status, excluded, partial,
  selected, detail), `log: HistoryLog` (commits, selected, detail), `sync: RefreshState`, plus `history`, `stashes`, `pr`,
  `merge`, branches and in-flight ops.
  Every git call goes through `cx.background_spawn`, and results are applied with `this.update`.
- **Stale results are dropped** with a `LoadSlot { generation, task }`
  (`repo/run.rs`): `RepoModel::load` (git read, errors go to `error`) or the generic
  `spawn_load` (also used by `Workspace` for the identity and merge preview loads)
  drops the previous task and ignores results of an older generation. The commit detail
  uses `LoadSlot::late()`: its loads are detached on purpose (holding an arrow key
  shows every commit) and late results still apply unless a newer one landed.
  **`DiffDetail`** (`repo/diff_detail.rs`) is the one store of files, selected file, `base`
  (commit/stash sha or PR merge base the files belong to), diff, loaded `path`, highlight and
  file-list scroll handle, for `RepoModel::{work.detail, log.detail}`, `stashes.detail`
  and `pr.detail`; one `HighlightCache` (`RepoModel::hl_cache`) serves all of them. `show_diff`
  is the only highlight spawner (a `LoadSlot` per detail), `show_files`/`load_files`/`select_file`
  the only file-loading paths. `components/file_diff_pane.rs` renders header + file list + diff for
  commit, stash and PR. Clicking quickly through files never shows the wrong diff.
  A status refresh always reloads the selected diff (status list equality is not
  enough: re-editing a modified file leaves it identical); `apply_change_diff`
  drops results equal to what is shown.
- `RepoModel::run(op, work, on_ok, cx)` (`repo/run.rs`) is the one write-op runner. It runs
  off-thread, sets `error`, then refreshes. `Op { label, committing, remote }` names the
  flags the op marks in flight; `run` sets and clears exactly those. A `label` fills
  `merge.busy` (banner text), allows one labeled op at a time and clears the history
  multi-selection. Unlabeled ops (commit, ignore, discard, checkout, stash ops, fetch/pull/push)
  may overlap.
- Refresh triggers: file watcher, window activation (skipped if the watcher
  refreshed <1s ago), after every op, F5. See §File watcher.
- UI lives in `features/<feature>/` (see CONVENTIONS.md): `RenderOnce` components, with
  `impl Workspace` only where `&mut Workspace` is needed. Rows in lists hold
  `Entity<RepoModel>` clones and call `update` in click handlers.

### List keyboard navigation (`workspace/navigation.rs`)

- ↑/↓/Home/End/PageUp/PageDown are bound in the `Workspace` context (`SelectNext/Previous/First/Last/PageUp/PageDown`); `MENU` bindings are defined later so overlays win. Focused text inputs bind ↑/↓/Home/End themselves and `select_move` returns when something other than the root has focus, so typing is unaffected.
- Target = `Workspace.nav_list` (`NavList`: Changes, Commits, Stashes, `Files(DiffPane)`), set by `nav_focus(div, list, ws)` (capture mouse-down on each list container; also refocuses the root). Falls back to the tab's main list. Selection goes through the normal `select_*` methods (background diff load + generation counters = latest wins) and `scroll_to_item`.
- Snapshot views `nav-history`, `nav-stash`, `nav-changes` dispatch the real actions.

### Menus, dialogs, undo (`features/menus`)

- **One overlay stack**: `Workspace::overlays: Vec<Overlay>` (`workspace/overlay.rs`) holds every open
  menu, dialog, toolbar popover, diff-options popover and the settings modal; the last entry is the top and
  paints last (`render.rs::overlays()` is one `match` per entry). `open_overlay` replaces the stack, so
  openers never close the others by hand. The only real stack is `stack_overlay`: the branch context menu
  opened over the branch picker (Esc closes the menu, then the picker). Esc = `close_overlay` (pop + refocus
  the root; with nothing open it falls back to leaving the PR preview), Enter = `overlay_confirm` (menu row
  or the dialog's own confirm), ↑/↓ = `overlay_step` (menus). While anything but a picker is open the root gets
  key context `"Workspace Menu"`, so `MenuNext/MenuPrevious/MenuConfirm` and `Dismiss` reach the workspace.
  Picker cursors, the filter/name/url inputs and `dismissed` live in `PopoverStore` (`Workspace::pickers`, `features/pickers/store.rs`). The PR base dropdown lives on
  `RepoModel.pr.base_menu` and is not an overlay.
- **Adding a menu**: write `fn x_menu_rows(ctx) -> Vec<MenuRow<XAction>>` and `fn run_x_action(&mut self, action, ctx, ..)`
  in the feature, then `self.open_menu(Menu::new(position, rows, move |ws, a, w, cx| ws.run_x_action(a, ctx.clone(), w, cx)), w, cx)`.
  `components::Menu` binds rows to the closure and owns `step()`/`confirm()`; `MenuView` renders it. The menu is
  already popped when `run_x_action` runs.
- **Adding a dialog**: a variant on `Overlay` holding the feature's dialog enum, an arm in `overlay_confirm` and in
  `render.rs::overlays()`. Build it with `gd_ui::Dialog`, which handles Enter/Esc even while an input is focused
  (`on_confirm` / `on_cancel`).
- Right-click handlers in `uniform_list` rows capture `cx.entity()` of the Workspace and call
  `ws.update(cx, ..)` with the row index, resolved against the live model at event time.
- `RepoModel.last_commit` records the session's commit; `undoable_commit()` hides it when the branch
  differs. Undo emits `RepoEvent::CommitUndone`, which the Workspace subscribes to in order to refill the inputs.
- Setting Workspace fields directly (snapshot code) needs `cx.notify()`.

### Performance rules (primary goal)

1. Never run git on the UI thread.
2. Any list that can be long must be a `uniform_list`: file lists, history, diffs, pickers.
3. History is paged (`HISTORY_PAGE` = 200). More pages load as the list nears its end.
4. Animations are one-shot. Don't use `repeat()` animations for idle UI. The only
   repeating one is the toolbar busy pulse, and it exists only during network ops.
   (zeron's `motion.rs` notes that a repeating animation pins the window at 120 Hz.)
5. The dev profile builds deps at `opt-level=3` so debug runs are representative.
6. Release profile: thin LTO, `codegen-units=1`.

Measuring idle cost: `GD_DEBUG_FRAMES=1` logs `renders=N moves=M` once per second
while the root view renders (`app/src/frame_debug.rs`). Silence means truly idle.
Renders far above pointer moves mean something requests frames on its own.
Don't measure with raw CPU ticks on a live desktop: pointer movement over the
window legitimately re-renders (hover transitions), which made one session chase
a phantom regression.

### Commit box (`features/changes/commit_box.rs`)

`Workspace::commit_box: Entity<CommitBox>` owns the message input, the amend state (`amend` root), the
placeholder layout-cache workaround (`sync_placeholder`, the one place it is documented) and renders
the amend notice + commit button. It observes the `RepoModel` itself (`attach` on repo switch) and reads
`Workspace::sidebar_width()` through a `WeakEntity` for the input width. `Workspace` reaches it through
`commit_box.read(cx).text(cx)` / `update(.., set_text | clear | start_amend | stop_amend)`.

Typing now notifies only `CommitBox`, never `Workspace` (test: `typing_does_not_notify_workspace`,
`cargo test --features snapshot`). **But `Workspace::render` still runs once per keystroke**, measured
in that test: gpui-ce re-renders the (uncached) window root on every draw, and `cx.notify()` on a view
also dirties all its ancestors, so even a cached `Workspace` root re-renders (tried; no change). The
only lever left is to skip the layout/prepaint/paint of heavy siblings with `Entity::cached(style)`,
which needs a definite size and the diff pane / file lists as entities of their own (the diff renderer
needs `&mut Workspace` today). Not done.

## File watcher and refresh coalescing

- `RepoModel::new` starts `RepoWatcher` (`watcher.rs`): a `notify-debouncer-full` debouncer
  (owns the thread, idle = no polling; 300ms per-event delay, no max wait, so a long burst yields a
  few ticks rather than one). The handler filters events and sends one tick (`bool` = git
  metadata changed) over a `smol` channel to a foreground task in the model.
  Dropping the model drops `RepoWatcher`, which drops the debouncer (thread stops, watches go away).
- Watches: root non-recursive, each top-level dir recursively (except `.git` and what the
  repo's `.gitignore` / `info/exclude` ignores, via the `ignore` crate; nested `.gitignore`s are
  not read), git dir non-recursive + `refs/` recursive. New top-level dirs get a watch on their
  Create event (the handler `try_lock`s the shared debouncer). Not one recursive root watch:
  that would cover `.git/objects` and build dirs.
- **Gotcha:** inotify reports `Access(Open)` for reads, and our own `git status`
  opens every file and `.git/index`. Only Create/Modify/Remove/Any events count,
  otherwise refresh retriggers itself forever. `GIT_OPTIONAL_LOCKS=0` keeps status from writing the index.
- `RepoModel::request_refresh(scope, cx)` (`RefreshScope::{Status, Full}`) is the only refresh path: if one is in
  flight it records `pending_refresh` and exactly one follow-up runs afterwards
  (`full` is OR-ed). Status-only refreshes (worktree edits) skip branches/history.
  `refresh_count` + a debug log line per refresh make loops visible.
- Last fetch = mtime of `<common git dir>/FETCH_HEAD` (read in each refresh).
  Background fetch task: first check at 10s, then wakes every 60s (also re-renders
  the relative-time caption); fetches when both last fetch and last attempt are >=30 min old.
  Failures are only logged.
- Headless test (`cargo test -p desktop-rs --features snapshot watcher`, `tests/watcher_tests.test.rs`): the watcher debounces
  in wall-clock time, so use real sleeps and poll `run_until_parked`.

## History (`gd_git::history_ops`, `app/src/features/history/`)

- Layout: `state.rs` (`HistoryState` on `RepoModel.history`, `HistoryUi`, `HistoryDialog`, `HistoryAction`), `ops.rs` + `selection.rs` (`impl RepoModel`), `handlers.rs`/`menu_actions.rs`/`confirm.rs`/`amend.rs` (`impl Workspace`), `commit_detail.rs` (`impl Workspace`, composes `CommitHeader`/`FileList`/diff), and one `RenderOnce` per file (`CommitList`, `CommitRow`, `commit_menu_rows`, `HistoryDialogView`, `MultiSelectPane`, ...). Commit, stash and PR file lists are the shared `components/FileList` (rows: `components/FileRow`).

- State: `RepoModel.history` (unpushed set, tags by sha, GitHub web URL, in-progress op, busy label,
  multi-selection as shas) reloaded in the background by `reload_push_state` on every full refresh.
  UI state: `Workspace.history_ui` (dialog text inputs; amend mode lives in `CommitBox`); the menu and dialogs are `Overlay`s.
- Every op goes through `RepoModel::run` with a labeled `Op` (one at a time, then `refresh`). Menu items are
  disabled while an op, remote op, commit, or stopped merge/rebase/cherry-pick/revert is active.
- Unpushed: `rev-list @{upstream}..HEAD`, or `HEAD --not --remotes` without upstream; empty with no remotes.
- Squash: adjacent, linear commits only. The squashed commit is built with `commit-tree` (newest tree, oldest's
  parent). If the run ends at HEAD, `update-ref` moves the branch (working tree and index untouched).
  Otherwise `rebase --autostash --onto <new> <newest>` replays the later commits; they apply cleanly
  because the tree is unchanged.
- Reword (non-HEAD; HEAD uses Amend): `reword_commit` builds the new commit with `commit-tree` (same tree and parents, original
  author via `GIT_AUTHOR_*`), then `update-ref` at HEAD or `rebase --autostash --onto <new> <old>` for the later commits.
  Dialog `HistoryDialog::Reword` reuses the squash message inputs (`HistoryUi::msg_summary/msg_desc`). Warns first on pushed commits.
- Force push: `WorkingStatus::rewritten` = diverged (ahead and behind) and the upstream tip appears in the branch reflog,
  i.e. we rewrote our own history (a teammate's push is not in the reflog, so that stays a Pull). The toolbar button then
  shows `RemoteOp::ForcePush` (`push --force-with-lease`). `Workspace::remote_action` never runs it directly (nor a plain Push
  while `rewritten`): it opens the preview modal (`Workspace::open_force_push` -> `Overlay::ForcePush`),
  `features/force_push/` (`RepoModel::force_push: ForcePushState`, `DiffPane::ForcePush`). `Repository::force_push_preview` (from the last
  fetch, what `--force-with-lease` checks) gives the commits to push / dropped from the remote and the net file changes remote tip -> HEAD
  (`files`, `stats`); the modal shows them with a per-file diff (`render_file_diff_pane` inside `gd_ui::modal`) and a Cancel / Force push
  header (`Workspace::confirm_force_push` closes it, then runs `RemoteOp::ForcePush`). Enter does nothing on purpose. Esc/backdrop close it.
  `RepoModel::force_push.open` mirrors the overlay (`sync_force_push_open`, called from `stack_overlay`/`close_overlay`) so refreshes and
  diff-option changes reload it.
- Reorder: `move_commit` (swap with neighbour) writes a todo file into `.git/gd-rebase-todo` and runs
  `rebase -i` with `-c sequence.editor="cp '<file>'"`; removed afterwards. No drag and drop (uniform_list
  drag with autoscroll not attempted); menu "Move commit up/down" instead.
- Revert/cherry-pick/squash-rebase/reorder return `OpOutcome` via `outcome_of`; on conflicts the op stays in progress and the generic in-progress banner (Continue/Abort) takes over. No private abort helpers.
- Amend: `amend_commit` resets the index, adds the checked paths, `commit --amend --allow-empty -F -`.
  Undo reuses `undo_last_commit` after seeding `last_commit` from HEAD, so the message returns via `CommitUndone`.
- Gotchas: `Focusable` must be imported for `Entity<EditableTextState>::focus_handle`; `gd_ui::tooltip(text)` is the
  tooltip builder (element needs an id); `Dialog::content` takes custom content (inputs, lists).

## Merge, rebase and conflicts

- `gd-git`: `progress.rs` (`Repository::in_progress() -> Option<InProgress>`: Merge/Squash/Rebase/CherryPick/Revert, read from MERGE_HEAD, `rebase-merge/`, `rebase-apply/`, CHERRY_PICK_HEAD, REVERT_HEAD, SQUASH_MSG+unmerged; `continue_operation`, `abort_operation`, `prepared_message`) and `merge.rs` (`merge`, `merge_squash`, `rebase`, `merge_preview`, `commit_merge`, `resolve_with`, `mark_resolved`, branch rename/delete-remote/create-from). Operations return `OpOutcome::{Done, UpToDate, Conflicts}`; a pause is not an error. `Git::no_editor()` sets GIT_EDITOR=true for `--continue`.
- The watcher already covers `rebase-*` (git dir top level) and MERGE_HEAD etc.
- App: `RepoModel.merge: MergeState` (`features/merge/state.rs`, ops in `features/merge/ops.rs`), loaded in every refresh in the background. `RepoModel::run` with a labeled `Op` is the busy-flag write helper. A newly paused merge-like op emits `RepoEvent::PreparedMessage` (fills the commit box if empty). While merge-like ops are paused, `commit` uses `commit_merge` (no index reset).
- Views: `features/merge/` (`MergeUi` = `Workspace.merge_ui`: `merge_dialog` + `merge_candidate_list` + `merge_preview`, `in_progress_banner`, `conflict_hint` above the diff, `conflict_menu_rows`, `handlers`/`conflict_handlers`) and `features/branches/` (`BranchUi` = `Workspace.branch_ui`: `branch_menu_rows`, `branch_dialog` for rename/delete/create-from, `ops`, `handlers`). Esc/Enter/arrows go through the overlay stack. Conflicted file rows route to the conflict menu in `open_file_menu`.
- **Hooking another op (revert, cherry-pick, squash) in:** run the git command inside `RepoModel::run`, map a pause with `Repository::outcome_of`, and use `notice_for_outcome`. Banner, conflict list and Continue/Abort work automatically.
- Gotchas: absolute overlays must themselves be `.absolute().inset_0()` (GPUI lays absolute children out against their direct parent); a focused text input turns ↑/↓/Esc into `NavUp/NavDown/Escape` actions, so dialogs use `capture_action` for those; `gen` is a reserved word in edition 2024.

## Syntax highlighting (`crates/syntax`, `app/src/features/diff/highlight.rs`)

- `gd-syntax` is ported from zeron's `zeron-syntax`, trimmed to Rust, JS/JSX/TS/TSX, Python,
  Go, JSON, Bash, TOML, Markdown (inline styles only, no fenced-code injection), HTML (+JS/CSS
  injection), CSS, YAML, C, C++. `highlight(HighlightRequest{source, path})` returns per-line
  `(byte_range, HighlightKind)`; unknown extension -> `UnknownLanguage`. `configuration()` is one
  table row per language. Size limits live in the app (`diff/highlight.rs`: 2 MB, 20k rows).
- Why tree-sitter over syntect: parse is incremental-grade fast (18k rows in ~125 ms), queries
  are shared with zeron, and the palette maps 1:1 to `HighlightKind`. Cost: ~16 grammar crates
  of C code (longer cold build, a few MB of binary).
- `gd-ui::theme::SyntaxPalette` (7 base colors, fixed per dark/light and independent of the color theme, mapped in `color()`: indigo keywords/functions, emerald strings,
  amber numbers/types, pink macros). Variable/parameter/operator/punctuation are drawn in the
  row's own color (no run) to cut noise and run count.
- App flow: after a diff loads, `RepoModel::show_diff` runs `highlight_diff` via
  `background_spawn`. Until it lands the view renders plain text; then `DiffDetail::hl`
  is set and the row switches to `StyledText::with_runs` (same metrics, so no layout shift).
  Staleness: the highlight `LoadSlot` of the detail is restarted on each new diff, so an older result is dropped. Results are cached in an 8-entry LRU keyed by
  hash(path, diff text), shared with the background task.
- Both sides are highlighted separately (context+delete = old, context+add = new; context and
  adds take new-side spans, deletes old-side spans). Hunks are stitched with a blank line, so a
  comment/string spanning an unchanged gap may be mis-colored (full-file highlight would fix it).
- Caps: skip above 20k diff rows or 2 MB text. Per-row runs are built only for visible rows.
- Gotcha: `StyledText::with_runs` panics if run
  lengths don't sum to the text length / split a char; `runs_for` guards both.
- Gotcha: `Hsla` here is palette `Alpha<Hsl>`: use `color.color.saturation`, not `.s`.
- Scroll reset: `Workspace::diff_key` stores the last diff identity; a change resets the list's
  `base_handle` offset and `DiffView::hscroll` to 0.

## Diff options and split view (`features/diff/`: `options/menu.rs`, `list/split_model.rs`, `list/split_rows.rs`)

- Options are app-wide and persisted in `Settings` (`diff_split`, `diff_ignore_whitespace`, `diff_wrap`), stored in
  `settings.toml` (serde + toml, `#[serde(default)]` so missing keys are fine).
- `gd_git::DiffOptions { ignore_whitespace }` is passed to `working_diff`/`commit_diff` (`-w`).
  `RepoModel::diff_options` holds it; `set_diff_options` reloads the visible change/commit diff via the
  normal generation counters (old diff stays on screen until the new one lands). `Workspace::apply_diff_options` pushes settings in;
  `open_repository` seeds new models.
- `Workspace::diff_view: DiffView` holds the cached `SplitDiff`
  (keyed by `Arc::ptr_eq` on the diff). `SplitDiff::build` is one linear pass producing
  `SplitRow { left, right }` index pairs into `Diff::lines` (`NONE` = filler): context on both sides, hunk
  headers span both columns (`left == right`), delete/add runs are paired row by row and padded.
  Rendering is a single list (id `diff-split`); each row is `[old text | middle gutter | new text]`.
  Syntax spans reuse `DiffHighlight::row(ix)` unchanged.
- Long lines (`list/text_flow.rs`, `TextFlow`): both modes share the `UnifiedRows`/`SplitRows` row renderers and
  `rows_list` (`list/rows.rs`), which picks the list type.
  - Scroll (default): a vertical-only `uniform_list`. Horizontal scroll is done by hand: each text cell
    (`text_cell`) clips and shifts its text by `DiffView::hscroll`. Gutters stay fixed and both split
    columns always move together. `diff_body`'s `on_scroll_wheel` takes the x delta (shift+wheel already
    arrives as x from the platform) and calls `Workspace::scroll_diff_x`. That clamps the offset using the
    shaped width of `diff.longest_line` and `HExtent` (non-text width + column count, set each render).
    The `uniform_list` gets `restrict_scroll_to_axis` (set via `interactivity().base_style`, since
    UniformList isn't `StatefulInteractiveElement`); otherwise a y-only scroller turns x deltas into
    vertical scrolling.
  - Wrap ("Wrap long lines"): a variable-height `gpui::list` on `DiffView::wrap_list`. `wrap_list_for`
    resets it when the diff `Arc` or split mode changes, and keeps the top row on same-key reloads. The
    list re-measures on width change by itself. Rows use `min_h`, and text uses `whitespace_normal`.
  - Gotcha: no `h_full` on gutter cells. A percentage height can't resolve in an auto-height (wrapped)
    row and blocks flex stretch. The cells stretch instead, are top-aligned, and keep check marks in a
    one-line-tall box.
- The popover is a `deferred` overlay like `ContextMenu` (`Overlay::DiffOptions`; closes on backdrop click/Esc).
- Gotcha: a `SplitRow` side can be `NONE`; never index `lines[left]` before checking `left == right`.
- Snapshot views set settings fields directly (no file writes).

## UI kit (`crates/ui`)

- `theme.rs`: the token struct. Use `Theme::of(cx)` and `theme.ink(a)` for translucent washes (white on
  dark, black on light, decided by `theme.light`). Never hardcode `white()`/`black()`; use theme tokens or
  `theme.ink()`, and check new UI with both `changes` and `changes-light` snapshots.
- Color themes (`themes.rs`): JSON files (default, Nord, Tokyo Night, VS Code, Catppuccin, Solarized embedded from
  `crates/ui/assets/themes/<id>.json`). Each family is `{"id", "label", "dark": {..}, "light": {..}}` with a `Palette` of
  14 `"#rrggbb"` colors per variant (see `default.json` for the field names); syntax colors are not part of a palette: `SyntaxPalette::new(light, text)` gives one fixed dark and one fixed light set, only the plain code `text` follows the theme;
  `Theme::from_palette` (the only constructor, default included) derives washes/diff colors (a palette may override them with optional `diff_*` keys, see `docs/THEMES.md`). The System/Light/Dark mode
  picks the variant, so every family needs both. Add a built-in theme = copy `default.json` (the template) to `assets/themes/<id>.json`
  (the `embedded_themes_parse` test covers it). `docs/theme.schema.json` is the JSON Schema for editors, generated from the
  Rust types with `schemars`; regenerate it after changing `ColorTheme`/`Palette` with
  `UPDATE_SCHEMA=1 cargo test -p gd-ui checked_in_schema` (the test fails when it is stale). A top-level `"$schema"` key is accepted and ignored.
  User-facing format and folder: `docs/THEMES.md`. User themes in `<config dir>/themes/*.json` replace a built-in with
  the same `id`; a file that fails to parse is logged and skipped. `ColorThemes` (a GPUI global, built-ins in `gd_ui::init`, user themes added in `main.rs`)
  is the registry: `get(id)` falls back to default. Persisted as `color_theme=<id>`; `Workspace::apply_theme` calls
  `Theme::for_family(id, light, cx)`. Snapshot view `theme:<id>:<dark|light>[:settings]` (built-ins only).
- `motion.rs`: the zeron motion catalog (`fade_in` 500ms ease-out-quint with 4px rise,
  `fade_quick`, `menu_in`, `dialog_in`).
  Overlay hand-off: overlays call `motion::handoff(id, cx)` (and `Workspace::render` calls
  `motion::begin_frame`); an overlay that first appears while another was drawn on the previous frame
  (menu -> dialog, popover -> popover) skips its entrance and scrim fade, so nothing flashes. New overlays must do the same.
- `components/` (one primitive per file, re-exported at the crate root): `pressable` and the small style fns in
  `surface.rs` (`badge`, `overlay_card`, `divider`/`vdivider`, `icon`, `empty_state`, `mixed_mark`), `Button`,
  `Checkbox`, `CheckRow` (checkbox row with a callback), `OptionRow` (radio card), `Tabs`, `ToolbarButton`,
  `ResizeHandle` (drag payload + double-click reset, right-edge strip or bottom grip), `Callback`/`ClickHandler`
  (the stored-callback type aliases).
- Overlays are stateless `RenderOnce` wrapping a `deferred` full-window layer; the owner renders them only while
  open and handles keys. `Popover` (click-away backdrop, optional scrim, card at a window position, `menu_in` +
  `handoff`) is the base of `ContextMenu` (positioned at the mouse, clamped to the window; icon/danger/disabled
  items, separators, `.selected(ix)`), `PopoverLayer` and `DiffOptionsMenu`. `Dialog` (scrim + `dialog_in` card,
  Cancel + confirm, Enter/Esc captured while an input is focused) and `modal(card, on_cancel, instant)` (the scrim + centered card used by the settings modal).
  The `popover_dismissed` workaround (a backdrop click must not re-trigger the toolbar button) stays on
  `Workspace` because it needs to know which popover the trigger opens.
- App-side components used by 2+ features are in `app/src/components/` (see CONVENTIONS.md).
- `assets.rs`: embedded Solar icons and Geist fonts. To add an icon, copy the
  SVG from `.ref/zeron/crates/ui/assets/icons/` into `crates/ui/assets/icons/`
  and add a line to the `icons!` macro. `rust-embed` embeds the folder.

## Platform code (`app/src/platform/`)

OS-specific code lives in `platform/linux/` and `platform/macos/`; `platform/mod.rs`
re-exports the one for the target. Both expose the same surface: `Chrome`,
`caption_buttons`, `titlebar_drag`, `frame`, `configure_window`,
and the shortcut constants `SECONDARY` (ctrl/cmd), `SECONDARY_LABEL`, `REFRESH_KEY`,
`QUIT_KEY`. Keep OS `cfg`s out of the rest of the app; add to both modules instead.
The config dir comes from the `dirs` crate (`dirs::config_dir()`) in `features/settings/model.rs`.
Wayland/X11 gpui features are enabled only for Linux targets in `crates/app/Cargo.toml`.

- **macOS** (not yet compiled or run, no Mac available): native frame with a transparent
  titlebar, so the toolbar is the titlebar. `Chrome::leading_inset` (78px) clears the traffic
  lights, the empty toolbar space is a `WindowControlArea::Drag`, no caption buttons or CSD
  frame. Shortcuts use cmd (Refresh is cmd-r, Quit cmd-q); config is in
  `~/Library/Application Support/desktop-rs`.

## Window chrome / CSD (`app/src/platform/linux/chrome.rs`)

- Linux requests `WindowDecorations::Client` with `LinuxWindowBackground::Transparent`.
  GNOME/Wayland never draws a frame. If the compositor negotiates server
  decorations instead (KDE, X11 WMs), `Chrome::detect` reports `client=false` and nothing extra renders.
- Floating: a 12px transparent margin (`set_client_inset`) holds the shadow, then a
  10px-rounded, hairline-bordered window. Tiled/maximized: no margin, square corners.
- GPUI content masks are rectangles (`.rounded()` doesn't clip children), so every
  layer touching a window corner rounds its own background: the toolbar (`rounded_t`),
  the workspace root, and the sidebar (`rounded_bl`, via `Workspace::corner_radius`).
- The titlebar is the toolbar's empty flex spacer (`titlebar_drag`). It isn't the whole
  toolbar, because mouse-down would bubble from the buttons and start a window move.
  Caption buttons stop propagation and follow `cx.button_layout()` (left and/or right).
- Window-coordinate positions (mouse events) must subtract `window.client_inset()`
  before absolutely positioning inside the frame (see `ContextMenu`).
- `GD_SNAPSHOT_CSD=1` forces the floating CSD frame in `--snapshot` renders.

## GPUI-CE gotchas (learned the hard way)

- **`AssetSource::list("")` must enumerate every asset.** GPUI-CE builds its
  `AssetRegistry` from it once at startup. If an asset isn't listed, `svg()`
  silently draws nothing.
- Colors are `palette` types. `.opacity()` needs `use gpui::ColorExt`. Build
  colors with `gpui::rgb_to_hsla(gpui::Rgba::new(r,g,b,a))`, not struct literals.
- `with_animation` needs `use gpui::AnimationExt`.
- Style transitions: `.transitions(|t| t.bg(millis(120)).text_color(..))` plus
  `.hover_style(..)` / `.active_style(..)`. Transitionable properties include
  `bg`, `border_color`, `text_color`, sizes, insets, and ring.
  `.hover(..)` and `.group_hover(..)` are deprecated aliases of the `_style` forms.
- `.flex_shrink(f32)` takes an argument.
- `uniform_list(id, n, |range, window, cx| ..)` has no entity parameter. Capture an `Entity` clone.
- Edition 2024: `-> impl IntoElement` on `&mut self` methods captures the borrow.
  Use `-> impl IntoElement + use<>` (only when no APIT args), or return `AnyElement`.
- Text inputs: `gpui_elements::editable_text::{text_input, text_area, EditableTextState}`.
  The state is an entity (`as_str()`, `emplace()`). Subscribe to `TextChanged` to re-render.
  Bind `default_bindings()` for `DEFAULT_INPUT_CONTEXT`. Catch Enter with
  `.capture_action(|_: &editable_text::actions::Enter, ..|)` on a wrapper.
- Headless snapshots run on a fake dispatcher clock, but **animations use wall-clock time**.
  the harness (`snapshot.rs`) sleeps for real and refreshes before capturing. Without that,
  fades are captured at opacity 0.
- `import`/`grim` aren't available on the dev machine. Use `--snapshot` to see the UI.

- **Auto-height wrapping text inputs need a definite width.** With `w_full` gpui-ce also
  measures the editable text at max-content width and its layout cache then paints that
  unwrapped layout. `CommitBox` passes the sidebar width in pixels instead.
- **Long words are broken by a vendored `gpui_ce_parley`** (`vendor/gpui_ce_parley`, wired up
  by `[patch]` in the root `Cargo.toml`). It is the pinned rev's crate plus one line:
  `OverflowWrap::Anywhere` in `text_system.rs`. Re-vendor it when bumping the GPUI-CE pin.
- **A `div` defaults to `Display::Block`, which lays its text children out as one inline
  paragraph.** The child `StyledText` is then never measured itself, so its `TextLayout`
  stays empty and `index_for_position` / `position_for_index` panic ("measurement has not
  been performed"). Give a text you hit-test a flex parent (`line_text` uses `flex().flex_col()`).
- gpui-ce only scrolls an input to the caret on caret movement, so typing past the commit box's
  max height does not follow the caret (see the reverted `c22f88d`).

## GPUI-CE pin

`Cargo.toml` pins `rev = 9a82fba4…` (2026-10-18). To bump it, change the rev
in all four entries, run `cargo update -p gpui-ce`, rebuild, and re-check the
gotchas above.

## Diff text selection (`features/diff/text_selection/`: `model.rs`, `mouse.rs`, `handlers.rs`)

- Model `TextSel { anchor, head, side }` in (index into `Diff::lines`, byte offset) coordinates, stored in
  `DiffView::text_sel`, so it is independent of virtualization. Split view selects within one `TextSide`; lines of a
  side stay monotonic in `diff.lines` order. Cleared on file change and split toggle.
- Hit-testing: `line_text` returns the row's `StyledText` `TextLayout`; the row closures capture it and call
  `index_for_position` (window coordinates, so h-scroll and wrapping just work). The text must sit in a flex parent
  or its layout is never measured (see the GPUI-CE gotchas). Mouse down is bound on the text cell
  only (gutters keep line staging); `on_mouse_move` on the whole row extends the drag (`text_drag`, cleared in
  `DiffFrame` mouse-up-all).
- Highlight = background on the selected `TextRun` range (`Theme::selection`), no extra paint elements.
- Ctrl/Cmd+C / +A are `CopyDiffText` / `SelectAllDiffText` on the `Workspace` context; a text press focuses the
  workspace so inputs (which bind their own copy/select-all deeper) do not intercept.
- Shared `CARGO_TARGET_DIR` across worktrees can mix `gd-ui` artifacts of different branches (spurious E0061); use a
  per-worktree target if you see impossible errors.

## Secret storage (accounts)

`features/accounts/secrets.rs` has `get/set/delete` by account id (blocking; `smol::block_on` around `oo7`
on Linux, `keyring` with native backends only on macOS/Windows). If the store errors (no Secret Service agent),
it flips `PLAINTEXT` and uses `<config dir>/secrets.toml` (0600 on unix); `plaintext_path()` drives the
warning in the Accounts list. Never add other fallbacks.

## OAuth sign-in and token refresh (accounts)

GitHub, GitHub Enterprise, GitLab, Gitea and Forgejo accounts sign in with the browser (`accounts/oauth.rs`,
`oauth2` crate with the `ureq` client; no reqwest/tokio). The user never types a token, except where no shared app exists (token fallback below). Bitbucket and Generic keep the token field.

- GitHub / GHE: device flow, client id only. The form shows the code (a read-only input, selectable) and opens
  the verification URL. Scopes `repo workflow read:user`.
- GitLab / Gitea / Forgejo: authorization code + PKCE (public client), redirect `http://127.0.0.1:47123/callback`
  (fixed port, `tiny_http` listener, one request, `state` checked). GitLab scope `api`; Gitea/Forgejo
  `read:user write:repository` (unverified against a live server).
- The flow runs on a `std::thread` and reports `Progress` over a `smol` channel; `AccountsStore::start_signin`
  applies it to the form. Cancelling the form (or switching provider) sets the stop flag of the loopback
  listener and makes later results stale (ignored). The device flow polls by hand (`oauth.rs`): GitHub answers a pending poll with HTTP 200 + `error`, which the oauth2 crate fails to parse.
- The only API call is `/user` (GitHub: `api.github.com` or `https://<host>/api/v3`; GitLab `/api/v4`;
  Gitea/Forgejo `/api/v1`) to fill the username. Hosts are assumed to be `https`.
- The add form has no username or display name field (OAuth providers; Bitbucket/Generic keep Username). The username always comes from `/user`, the display name defaults to it (editable when editing). The Host field shows only for providers without a fixed host.
- The account type is a compact 220px select (`provider_select` in `account_form.rs`): the open menu is a deferred card whose selected row sits over the control (like a native select), with a huge transparent backdrop in the same deferred element that closes it on any click (`on_mouse_down_out` would reopen it when the control is clicked).
- "Sign in with" is a radio group of `OptionRow`s (`SignInMethod` in `model.rs`: Browser only when `oauth_app(host, "")` exists, Personal access token, Your own OAuth app); only the chosen method's body shows. Default = `SignInMethod::default_for`; editing preselects the method the account used (secret's client id). A pasted token is checked with `/user` on "Add account" ("Checking...", "This token was rejected").
- Client ids are public (no secret), so they are source constants in `accounts/model.rs`. `Provider::oauth_app(host, custom)`
  resolves: custom client id typed in the "Your own OAuth app" method > `BUILTIN_CLIENT_IDS` entry of the host (empty = not
  registered) > Gitea/Forgejo default app > `None`. `None` (GHE, self-managed GitLab, or gitlab.com while unregistered)
  means the form shows "Create token" (opens `https://<host>/settings/tokens/new?scopes=repo,workflow,read:user&description=desktop-rs`
  or GitLab `/-/user_settings/personal_access_tokens?name=desktop-rs&scopes=api`) and the token field; saving a pasted token
  with an empty username looks `/user` up first. Pasted tokens have no refresh token (`Account.oauth = false`).
- Gitea and Forgejo preregister public apps (`DEFAULT_APPLICATIONS`, `models/auth/oauth2.go` `BuiltinApplications`); we use
  git-credential-oauth `a4792ccc-144e-407e-86c9-5e7d8d9c3269`, redirect `http://127.0.0.1` (no path, any loopback port for
  public clients, http only, IP literal only). So that app uses `GITEA_REDIRECT_URI` `http://127.0.0.1:47123` and the
  listener accepts the path `/`; every other app uses `REDIRECT_URI` (`/callback`). Unverified: that the stored apps are
  really public (`ConfidentialClient` column default is TRUE, `Init` does not set it; git-credential-oauth works with them in
  practice) and that an instance has not disabled them (`DEFAULT_APPLICATIONS=`). codeberg.org and gitea.com use that default app
  until own apps are registered there (another project's registered app ids are deliberately not reused).
- github.com uses the OAuth App "desktop-rs" (`Ov23liWNVoyi5E8E5lb0`, device flow enabled; an OAuth App, not a GitHub App,
  which would need installing per org and expiring tokens with a secret to refresh). Maintainer TODO: register the gitlab.com
  application (non-confidential, scope `api`, redirect `http://127.0.0.1:47123/callback`) and paste its id in `BUILTIN_CLIENT_IDS`. Own apps on codeberg.org /
  gitea.com (public client, redirect `http://127.0.0.1:47123`) are optional.
- The secret per account is a JSON `TokenSecret` (`accounts/token.rs`: access/refresh token, `expires_at`, client id,
  token URL; bare strings from older versions still parse). `Account.oauth` marks signed-in accounts.
- `accounts::fresh_token(account_id) -> Option<String>` is blocking (secret store + network): it refreshes
  and saves when the token expires within 60 s. Refreshes are serialized (refresh tokens rotate). Git network ops use it
  too: `HostCredential.id` + `gd_git::set_token_source(fresh_token)`, called while building the env of a network command.
  Git user of GitLab tokens is `oauth2`; GitHub/Gitea/Forgejo use the account username.

## Account per repository (accounts)

`accounts/repo_account.rs`: `account_for_repo(accounts, picks, repo_path, remote_host) -> Option<&Account>` (re-exported from
`features/accounts`). A pick (`AccountsStore::repo_accounts`, repository path -> account id, saved in
`<config>/repo_accounts.toml`, never in `.git/config`) wins; otherwise the first account whose host (port ignored)
is the remote host. `Repository::remote_host()` (gd-git) gives the host of the default remote. The picker is
`repo_account_block.rs`, in the repository part of the identity section. The CI page calls `account_for_repo` and then
`accounts::fresh_token(&account.id)` from a background thread.

## Repository settings (`features/repo_settings/`)

- `Overlay::RepoSettings`, state in `RepoSettingsStore` (`Workspace::repo_settings`, observed like `accounts`): page, repository facts (`gd-git/info.rs`
  `Repository::info`, read in the background with `spawn_load`). Layout is `components::SettingsShell` (sidebar `NavItem`s, title bar, Esc captured even
  while an input is focused); `SettingsModal` uses the same shell. The shell owns the one scroll area of the body and pads inside it, so pages
  do not scroll themselves and scrolled content reaches the title divider and the bottom edge.
  The shell always plays the entrance animation and does not use `motion::handoff`, which would skip it when the modal replaces the repository
  button's context menu (a menu -> dialog swap is instant by design; a big modal with a backdrop should still fade in).
- Identity and account pages reuse `accounts::RepoIdentity` / `RepoAccountBlock` and `AccountsStore` (`load_identity(Some(repo))` runs on open and on
  every page switch). The app Settings → Accounts page only has the global identity (`load_identity(None)`).
- Entry points: `Workspace::open_repo_settings`, `open_repo_menu` (right-click on `ToolbarButton::on_right_click`), the `OpenRepositorySettings` action.
- Writes go through `RepoSettingsStore::write(work)`: background `gd-git` call (`remotes.rs`: `remote_list`, `add/rename/remove_remote`, `set_remote_url`,
  `set_remote_push_url`), then on success the open form (`Edit`) closes, the page reloads and `RepoModel::refresh` runs; on failure `error` shows inline.
  A new page adds an `Edit` variant for its form/confirmation instead of a new field. `FieldRow` lives in `components/`.
- Ignored files: `IgnoreFile::{Gitignore, Exclude}` (`state.rs`) gives the path (`common_git_dir()/info/exclude`); the texts are read once on open (`load_ignore`, never on reload so unsaved edits survive a write) and `save_ignore` writes through `write()`. `notice` holds the success text of the last write.
- Git config: `gd-git/config.rs` (`local_config` parses `--list -z` records `key\nvalue\0`; `add/replace/remove_config` match one value with `--fixed-value`, so regex characters are literal and other values of the key stay; `validate_key`). Rows are identified by key + value (`Edit::Config(entry)`). Not done: showing the global/system origin of a key.
## CI page (`features/ci/`)

A full page, not an overlay: `Workspace::body` returns `CiPage` instead of sidebar + main pane while `CiStore::open` (the toolbar button
left of the settings cog, Ctrl/Cmd+Shift+I; Esc closes it after the overlays). Only the toolbar stays. `Workspace::ci: Entity<CiStore>`
is observed like the other stores; `attach_model` calls `sync_ci` when a repository is opened while the page is open, so it shows the new repo.

- `state.rs` (`CiStore`), `target.rs` (`Target` = forge + `RepoRef` + account; `Problem` = every empty state, `Problem::describe`; `forge_of(Provider)`),
  `ops.rs` (resolve, list, details), `polling.rs`, `handlers.rs` (`impl Workspace`: `toggle_ci`, `sync_ci`, `back_ci`).
  Components: `ci_page`, `pipelines_header`, `problem_view`, `pipeline_row` (plain fn), `stage_dots`, `status_badge` (status -> label/icon/tone, tested), `duration`.
  `breadcrumbs` is the one breadcrumb bar of the pipeline and job pages (fixed 56px high, so it does not move between them; the list header's top padding
  is set so its title is on the same line). Its "Pipelines" crumb is `CiStore::show_list`, the back arrow and the `#n` crumb are `back`.
- Resolution (`CiStore::show`, background): `tracking_remote_url` -> `RepoRef::from_remote` -> `account_for_repo` -> `forge_of(provider)`.
  Bitbucket and Generic accounts are "not supported". Every request builds its `Client` with `accounts::fresh_token` on the background thread,
  so an expiring OAuth token is refreshed. Another repository resets the store (`epoch` bump drops late detail results); the same one keeps its cache.
- The list has no stages. Rows ask for their `pipeline()` detail only for the visible range (`request_details`, from the `uniform_list` closure
  through `window.defer`), at most 4 at a time, cached by pipeline id in `details`. A failed detail is remembered in `failed` (else the render
  would re-request it forever) and retried by the next poll. Without real stages (GitHub, Gitea) `load_detail` reads the workflow file from the
  working tree (`workflow_candidates`) and calls `stages_from_workflow`.
- Navigation: Esc, the ← button of the breadcrumb and the mouse Back button all call `CiStore::back` (job -> pipeline -> list -> close the page; the
  workspace root binds `MouseButton::Navigate(Back)` to `Workspace::dismiss`, which closes an open overlay first). Back pushes the screen it leaves on
  `forward`; the Forward button (`Workspace::forward_ci`) pops it. Opening a pipeline or job from the UI clears `forward`; a new repository resets the store.
- The log list is virtualized (uniform rows), so its bottom margin is `LOG_BOTTOM_ROWS` empty rows after the last line (`LogState::rows`).
- Selecting log text (`log_selection.rs`): the log reuses the diff's `TextSel` / `TextPos` model (`Unified` side, lines of `Log::lines`, kept in
  `LogState::sel`); `log_line` merges the selection wash over the ANSI highlights with `combine_highlights`, hit-tests with the line's `TextLayout`
  (the text must sit in a flex column, see `diff::highlight::line_text`) and each row's mouse-move extends a drag. Section headers still toggle on
  click and are not a place to start a selection. Ctrl/Cmd+C and +A (`copy_diff_text` / `select_all_diff_text`) act on the log while the page is open;
  the copy skips lines a collapsed section hides. The mouse-up that ends a drag is the workspace root's.
- Steps in the log: the header line of a top-level section (`depth == 0`: a GitLab script stage, a GitHub/Gitea group) is drawn as a tinted band with a
  divider above it, so the steps can be told apart; nested sections keep the plain hover row. Uniform rows rule out a gap between steps.
  The rows inside an open section get the same wash, so an opened step is easy to tell from the lines around it.
- Toggling a section is an accordion (`LogState::anim`, 200ms cubic ease-out, no fade): its body rows are drawn in a container under the header
  (absolute, `overflow_hidden`) whose height grows (opening) or shrinks (collapsing), and the lines below are pushed down by exactly that height.
  The list is laid out with the body hidden for the whole animation: opening keeps `visible` without the body and a timer commits it when the
  animation is over, collapsing commits at once. So every row that must be drawn is in the list's rendered range, and the commit moves nothing. A
  section with more than `MAX_ANIM_ROWS` (40) visible rows opens and collapses at once (its container would be taller than the window).
  GPUI gotcha: `uniform_list` lays every row out as its own root (`layout_as_root` + `prepaint_at`), and a root ignores `relative().top(..)`, so the
  offset of the lines below is on a child inside a flex wrapper. The harness waits for loads for longer than the animation, so to see a frame midway
  raise `LOG_ANIM` (to 20 s) for the run, with the `ci-job-slide` / `ci-job-slide-close` views.
- Job cards do not use `pressable`: its hover is a translucent wash, which over an opaque card fades through the page behind it (a dark flash). They use
  the opaque `surface_raised` / `surface_raised_hover` pair.
- `CiStore::refreshing` is true from `show` (the Refresh and Retry buttons, opening the page, switching repository) until the list it loads is in, or the
  account could not be resolved; both buttons are disabled meanwhile. It is not `loading`, which polls and "Load more" set too (the button would flicker
  every poll). A poll that was already loading when `show` asked for the list satisfies it.
- Polling (`polling.rs`): one task while the page is open, every 10 s when something in the list is unfinished, else every 60 s; skipped while the
  window is inactive, and coming back to the window polls at once. A poll reloads page 1 and merges it (older pages stay, rows the new page covers
  are replaced), and refetches the details that are not finished. Finished details are never refetched. Running status icons are static.
- Pipeline screen (`CiView::Pipeline(id)`, Esc goes back to the list): `pipeline_view` (breadcrumb, summary, "Open in browser", Pipeline | Jobs tabs),
  the graph is `stage_column` + `job_card` in a scrolling row with a hairline between columns, the table is `jobs_table`. It reads the same `details`
  cache as the list; opening a pipeline refetches it (a cached running one is stale), and polling refetches it while it is not finished.
  An `allow_failure` job that failed is drawn with the warning look (`status_badge::look(Success, true)`).
- Job screen (`CiView::Job(id)`): `job_view` (breadcrumb) = `log_view` (toolbar + `uniform_list`) + `job_sidebar`. The raw log is fetched and parsed
  (`gd_forge::parse_log`) on a background thread and kept as `Arc<Log>`; `LogState::visible` lists the line indices outside collapsed sections
  (`ops::visible_lines`; a section is collapsed when `Section::collapsed != toggled.contains(section index)`, so user toggles survive log refreshes).
  Lines are `StyledText` with highlights (`ansi.rs`: two 16-color sets chosen by `theme.light`, xterm cube, gray ramp, RGB; the theme has no
  terminal palette). No bold (no bold face in the mono font), no wrapping (lines are clipped; "Copy log" copies everything), no in-log text selection.
  Following: while the job runs (`LogState::follow`) every load scrolls to the end; "Top"/"Bottom" switch it. While a job is open and unfinished the
  poll interval is 5 s and each poll reloads the whole log (the APIs have no range requests) and the pipeline detail.
- Actions (`actions.rs`): `CiAction` (retry / cancel pipeline, retry / cancel / play job). `job_actions(job, caps)` picks the buttons of a job from
  its status and `Forge::capabilities` (hover icon buttons on graph cards, labelled buttons in the job sidebar); the pipeline buttons are
  "Retry failed" (failed or canceled) and "Cancel" (unfinished). `CiStore::run_action` runs on a background thread (one at a time, `acting` disables
  the buttons), then emits `CiEvent::Notice` / `CiEvent::Error`, which `Workspace::on_ci_event` turns into the usual banner, and refetches the pipeline
  and page 1 of the list. The server may still report the old status for a moment: the next poll corrects it (a pipeline or job screen polls even when finished).
