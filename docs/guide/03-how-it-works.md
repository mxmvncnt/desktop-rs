# 3. How the app works, step by step

Each section follows one thing you do in the app through the code. File paths are
relative to `crates/app/src/` unless they start with `crates/`.

## Starting the app

1. `main.rs` `main()` sets up logging. With `--snapshot` it renders a screenshot and exits
   (see [06-build-test-debug.md](06-build-test-debug.md)).
2. `gd_ui::init` loads fonts and the built-in color themes. User themes are loaded from
   `~/.config/desktop-rs/themes/`.
3. `actions::bind` registers every keyboard shortcut (`actions.rs`).
4. A window opens with a `Workspace` inside (`workspace/init.rs`, `Workspace::new`):
   - it creates the stores: `PopoverStore` (toolbar dropdowns), `AccountsStore`
     (accounts; this also reads all tokens from the keyring), `RepoSettingsStore`,
     `CiStore`, and the text inputs of every dialog;
   - it loads `settings.toml` (`features/settings/model.rs`, `Settings::load`);
   - it opens the last repository (`workspace/repos.rs`, `open_repository`).
5. Opening a repository creates a `RepoModel` (`repo/state.rs`, `RepoModel::new`), which
   right away starts three things: a first refresh, the file watcher, and a timer that
   fetches from the remote every 30 minutes (`repo/background_fetch.rs`).

## A refresh (how the screen stays up to date)

A "refresh" re-reads the repository from git. File: `repo/refresh.rs`.

```
something happened ──► request_refresh(scope)
  (file saved, window focused,        │
   F5, after any git command)         │ already refreshing? → remember "one more after" and stop
                                      ▼
                       background thread: git status, git branches,
                       "is a merge in progress?", last fetch time
                                      │
                                      ▼
                       finish_refresh: store the results, then reload
                       stashes, push status, history, PR preview...
                                      │
                                      ▼
                                  cx.notify() → redraw
```

- There are two sizes of refresh: `Status` (only `git status`, when files in your work
  tree change) and `Full` (also branches, stashes, history, when something in `.git`
  changes).
- Only one refresh runs at a time. If ten files are saved during a refresh, exactly one
  more refresh runs afterwards. That's what keeps the app at zero CPU when idle.
- The file watcher (`watcher.rs`) uses the `notify` crate. It watches the repository but
  ignores what `.gitignore` ignores, and ignores "file was read" events (git's own
  `status` reads every file, which would otherwise refresh forever).

## Life of a click: selecting a changed file

1. The file list row is built by `features/changes/change_row.rs`. Its click handler
   calls `repo.update(cx, |r, cx| r.select_change(ix, cx))`.
2. `select_change` (`repo/change_diff.rs`) stores the selected index and calls
   `load_change_diff`.
3. `load_change_diff` uses `self.load(..)` to run `repo.working_diff(file)` in the
   background. `working_diff` lives in `crates/git/src/diff.rs`: it reads the old version
   from git (`git cat-file`) and the new one from disk, and **computes the diff itself**
   (in `crates/git/src/engine.rs`, with the `similar` crate). It doesn't use `git diff`.
4. When the diff arrives, `show_diff` (`repo/diff_detail.rs`) stores it and starts syntax
   highlighting in the background (`features/diff/highlight.rs`, which calls
   `crates/syntax`).
5. The screen redraws. The main pane (`features/changes/changes_pane.rs`,
   `render_changes_main`) calls `Workspace::render_diff` (`features/diff/view.rs`), which
   picks the unified or split list and draws the visible lines.
6. When the highlight arrives a moment later, it redraws again with colors.

If you click B while A's diff is still loading, A's result is thrown away when it
arrives. That's what `LoadSlot` (`repo/run.rs`) is for.

The same `DiffDetail` struct (`repo/diff_detail.rs`) holds "file list + selected file +
diff + colors" for every place a diff shows: the Changes tab, a commit in History, a
stash, the PR preview and the force-push preview. Which one is meant is told by a
`DiffPane` value (`DiffPane::Changes`, `DiffPane::Commit`, ...).

## Life of a commit

1. You type in the commit box (`features/changes/commit_box.rs`, its own entity so typing
   doesn't redraw the whole window) and press Ctrl+Enter.
2. The `Commit` action reaches `Workspace::commit` (`workspace/repo_commands.rs`). It reads
   the message, picks a default summary if empty ("Update foo.rs"), and calls
   `RepoModel::commit`.
3. `RepoModel::commit` (`features/changes/ops.rs`) calls `self.run(Op { committing: true, .. }, ..)`.
4. `run` (`repo/run.rs`) marks "committing" (the button shows a spinner), runs
   `commit_partial` in the background (`crates/git/src/lib.rs`, with the line-staging part
   in `partial.rs`: reset the index, add the checked files, stage partially-selected lines,
   `git commit -F -`), then:
   - on success: runs the "on ok" code (remember the commit for the Undo strip, set the
     "Committed 3 files" notice);
   - on error: puts the git error text in `RepoModel::error` (the red banner);
   - in both cases: refreshes.

Every write operation in the app (stash, checkout, merge, push, revert, ...) follows the
same `self.run(..)` path.

## Menus and dialogs

Everything that floats is an `Overlay` (`workspace/overlay.rs`):

```rust
pub enum Overlay {
    Popover(Popover),       // toolbar dropdowns (repository list, branch list, ...)
    Menu(Menu),             // any right-click menu
    DiffOptions(..),        // the diff gear popover
    Settings, RepoSettings, // the two settings windows
    Discard(..), History(..), Stash(..), Merge(..), Branch(..),  // dialogs, one variant per feature
    ForcePush,              // force-push preview
}
```

`Workspace::overlays` is a list; the last one is on top.

- **Esc** → `Dismiss` action → `dismiss` (`workspace/actions.rs`) → `close_overlay`: remove the
  top one. With nothing open, Esc leaves the CI page or the PR preview instead.
- **Enter** → `overlay_confirm` (`workspace/overlay.rs`): a `match` that calls the confirm
  method of whatever is on top (`stash_confirm`, `merge_confirm`, ...).
- **↑/↓** → `overlay_step`: moves the highlighted row of a menu.
- **Drawing**: `workspace/render.rs`, `overlays()`, has one `match` arm per kind.

A right-click menu is the simple case. Example from stashes:

```rust
// features/stashes/stash_menu.rs: the rows
MenuRow::item(StashAction::Apply, "Apply (keep stash)", Icon::ArchiveUp),

// features/stashes/handlers.rs: open it with a closure that runs the picked row
let menu = Menu::new(position, stash_menu_rows(), move |ws, action, w, cx| {
    ws.run_stash_action(action, stash.clone(), w, cx)
});
self.open_menu(menu, window, cx);
```

A dialog takes more steps; see the recipe in [05-recipes.md](05-recipes.md#add-a-dialog).

## Keyboard navigation in lists

↑/↓/Home/End/PageUp/PageDown move the selection in "the list you last clicked"
(`workspace/navigation.rs`). Each list wraps itself with `nav_focus(div, NavList::X, ws)`
so a click tells the workspace "I'm the list the arrows should drive now". When a text
input has focus, the arrows go to the input instead.

## Network: fetch, pull, push and sign-in

- The toolbar's remote button (`features/toolbar/remote_button.rs`) calls
  `Workspace::remote_action` (`workspace/repo_commands.rs`), which calls
  `RepoModel::remote` (`features/toolbar/ops.rs`).
- Git network commands never ask for a password in a terminal (`GIT_TERMINAL_PROMPT=0`).
  Credentials come from:
  - your own git setup (credential helper, ssh-agent), or
  - an account added in Settings → Accounts. Those tokens are handed to the git layer
    (`gd_git::set_credentials`, `crates/git/src/credentials.rs`), which passes them to
    `git` as a temporary credential helper on each network command.
- Sign-in with the browser (OAuth) is in `features/accounts/oauth.rs`. Tokens are
  refreshed when they're about to expire (`features/accounts/token.rs`, `fresh_token`).

## The CI page

`features/ci/` is a full page that replaces the sidebar and diff while it's open
(Ctrl+Shift+I or the toolbar button). It has its own store, `CiStore`
(`features/ci/state.rs`), which is the cleanest store in the app: it doesn't need
`Workspace` for most of what it does.

1. `CiStore::show` (`ops.rs`) works out which forge (GitHub, GitLab, ...) and which
   account to use for this repository (`target.rs`).
2. It loads the pipeline list through `crates/forge` (plain blocking HTTP, run in the
   background).
3. `polling.rs` reloads every 10 s while something is running, every 60 s otherwise,
   and not at all when the window isn't focused.
4. Click a pipeline → `CiView::Pipeline`, click a job → `CiView::Job` (the log viewer,
   `log_view.rs`).

## Settings

- The settings window is `features/settings/settings_modal.rs`, one file per page.
- The saved values are the `Settings` struct (`features/settings/model.rs`), written to
  `settings.toml` with `serde`. A missing key in the file gets its default value, so
  adding a field never breaks old files.
- `Workspace::settings` holds the live copy. Changing a setting = set the field, call
  `self.settings.save()`, call `cx.notify()`.
