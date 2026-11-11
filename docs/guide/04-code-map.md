# 4. Code map: where is what

Paths are relative to `crates/app/src/` unless they start with `crates/`.

## "I see this on screen, which file do I open?"

| On screen | File |
|---|---|
| The whole window layout (toolbar, sidebar, main pane, overlays) | `workspace/render.rs` |
| Top toolbar | `features/toolbar/toolbar_bar.rs` |
| Fetch / Pull / Push button and its chevron menu | `features/toolbar/remote_button.rs`, `features/toolbar/handlers.rs` |
| "Last fetched 5 minutes ago" | `features/toolbar/last_fetched.rs` |
| Repository dropdown | `features/pickers/repo_picker.rs` (shared frame: `picker_shell.rs`) |
| Branch dropdown | `features/pickers/branch_picker.rs`, rows: `components/branch_row.rs` |
| "Create a branch" dropdown | `features/pickers/new_branch_form.rs`, "Based on" field: `new_branch_base.rs` |
| "Clone a repository" dropdown | `features/pickers/clone_form.rs` |
| Changes / History tabs at the top of the sidebar | `features/changes/sidebar_tabs.rs` |
| The sidebar itself (and its resize handle) | `features/changes/sidebar.rs`, `features/changes/resize.rs` |
| "N changed files" header + select-all checkbox | `features/changes/changes_header.rs` |
| A changed file row | `features/changes/change_row.rs` → `components/file_row.rs` |
| Right-click menu on a changed file | rows: `features/menus/menu_rows.rs`, actions: `features/menus/menu_actions.rs` |
| "Discard changes?" dialog | `features/menus/discard_dialog.rs` |
| Commit message box and Commit button | `features/changes/commit_box.rs` |
| "Committed just now · Undo" strip | `features/changes/undo_strip.rs` |
| "Amending the last commit" notice | `features/changes/amend_notice.rs` |
| "No local changes" screen with cards | `features/changes/no_changes.rs`, cards: `components/suggestion_card.rs` |
| The file path bar above a diff | `features/changes/file_header.rs` |
| The diff (any of them) | entry: `features/diff/view.rs`; unified rows: `features/diff/list/unified_rows.rs`; split rows: `features/diff/list/split_rows.rs` |
| Diff line numbers and the blue line-selection gutter | `features/diff/gutter/number_cell.rs`, `block_handle.rs` |
| "Expand" arrows in hunk headers | `features/diff/expand/buttons.rs` |
| Diff colors (syntax highlighting) | `features/diff/highlight.rs` (palette: `crates/ui/src/theme.rs`) |
| The gear menu above the diff (split, wrap, whitespace) | `features/diff/options/menu.rs`, contents: `components/diff_display_options.rs` |
| Stashes panel in the sidebar | `features/stashes/stash_panel.rs`, rows: `stash_row.rs` |
| A selected stash in the main pane | `features/stashes/stash_detail.rs`, `stash_header.rs` |
| History commit list | `features/history/commit_list.rs`, `commit_row.rs` |
| History search box | `features/history/search_box.rs`, logic: `search.rs` |
| Selected commit details | `features/history/commit_detail.rs`, `commit_header.rs` |
| Right-click menu on a commit | rows: `features/history/commit_menu_rows.rs`, actions: `menu_actions.rs` |
| History dialogs (reset, squash, tag, cherry-pick...) | `features/history/history_dialog.rs`, OK button: `confirm.rs` |
| "2 commits selected" pane | `features/history/multi_select.rs` |
| Merge / squash / rebase dialog | `features/merge/merge_dialog.rs` |
| "Merge in progress · Continue / Abort" banner | `features/merge/in_progress_banner.rs` |
| Strip above a conflicted file | `features/merge/conflict_hint.rs` |
| Right-click on a branch, rename / delete dialogs | `features/branches/branch_menu_rows.rs`, `branch_dialog.rs` |
| Pull request preview | `features/pr_preview/pr_preview_pane.rs`, `pr_header.rs` |
| Force push preview | `features/force_push/force_push_pane.rs`, `force_push_header.rs` |
| CI page | `features/ci/ci_page.rs`; list: `pipelines_header.rs` + `pipeline_row.rs`; one pipeline: `pipeline_view.rs`; one job: `job_view.rs` + `log_view.rs` |
| Settings window | `features/settings/settings_modal.rs`, pages: `appearance_page.rs`, `diff_page.rs`, `about_page.rs` |
| Settings → Accounts | `features/accounts/accounts_page.rs`, form: `account_form.rs` |
| Repository settings window | `features/repo_settings/repo_settings_modal.rs`, one `*_page.rs` per page |
| Red / green banner at the bottom | `features/notices/notice_banner.rs` |
| "Let's get started!" screen | `features/welcome/welcome_screen.rs` |
| Window corners, shadow, close/maximize buttons (Linux) | `platform/linux/chrome.rs` |
| Blur / transparency behind the window | `platform/linux/backdrop.rs`, `platform/macos/backdrop.rs` |
| A button, checkbox, tabs, tooltip, dialog frame | `crates/ui/src/components/` |
| Colors, sizes, fonts | `crates/ui/src/theme.rs`; color themes: `crates/ui/assets/themes/*.json` |
| Icons | `crates/ui/assets/icons/*.svg`, list: `crates/ui/src/assets.rs` |

**Tip:** to find the code for a piece of text you see, search for that text:
`rg "Undo" crates/`. Most labels are written literally in the code, but text
built with `format!` (e.g. `format!("Committed {}", ..)`) is not — search a
stable fragment. The full technique is in [08-find-and-debug.md](08-find-and-debug.md).

## The `app` crate, folder by folder

```
crates/app/src/
├── main.rs            Starts the app (or the screenshot mode).
├── actions.rs         The list of commands and their keyboard shortcuts, and the macOS menu bar.
├── watcher.rs         Watches the repository folder for changes.
├── snapshot.rs        Headless screenshot mode (only with --features snapshot).
├── frame_debug.rs     GD_DEBUG_FRAMES=1: logs how often the window redraws.
│
├── workspace/         The window. The "root component".
│   ├── state.rs         The Workspace struct: every field it holds.
│   ├── init.rs          Workspace::new: creates everything at startup.
│   ├── render.rs        The root template.
│   ├── overlay.rs       The stack of menus / dialogs / popovers; Enter and Esc routing.
│   ├── actions.rs       Connects each keyboard command to a method.
│   ├── navigation.rs    Arrow keys in lists.
│   ├── repos.rs         Add / open / remove / clone repositories.
│   ├── repo_commands.rs Commit, push, undo... forwarded to RepoModel.
│   └── pane_view.rs     The sidebar and main pane as separately cached views (performance).
│
├── repo/              The open repository's data. The "main store".
│   ├── state.rs         The RepoModel struct and its sub-structs.
│   ├── refresh.rs       Re-reading status/branches/history, coalescing refreshes.
│   ├── run.rs           `run` (write ops) and `LoadSlot` / `load` (reads where the latest wins).
│   ├── diff_detail.rs   DiffDetail: file list + selected file + diff + colors, shared by every diff pane.
│   ├── change_diff.rs   Loading the diff of the selected changed file.
│   ├── commit_diff.rs   Loading the files of the selected commit.
│   ├── history_log.rs   Loading history 200 commits at a time.
│   └── background_fetch.rs  Fetch every 30 minutes.
│
├── features/<name>/   One folder per feature. Usual files:
│   ├── mod.rs           Lists the files and what the rest of the app may use.
│   ├── state.rs         The feature's data types.
│   ├── ops.rs           Methods added to RepoModel (the git work of the feature).
│   ├── handlers.rs      Methods added to Workspace (open/close dialogs, run menu actions).
│   ├── snapshot.rs      Screenshot test views for this feature.
│   └── *.rs             One component per file.
│
├── components/        Components used by two or more features (file row, file list, menu, combo box, ...).
├── utils/             Small non-UI helpers: text.rs (plurals, relative time...), paths.rs (open/reveal files).
└── platform/          OS-specific code: linux/ and macos/ expose the same functions.
```

## The other crates

### `crates/git` (gd-git): everything git

| File | What |
|---|---|
| `lib.rs` | `Repository` (open, init, clone, commit, discard...) and the list of everything exported |
| `exec.rs` | The one place that starts the `git` program (`Git::new(dir).args([..]).run()`) |
| `status.rs` | `git status` parsing |
| `diff.rs`, `engine.rs` | Loading both versions of a file and computing the diff |
| `partial.rs` | Committing only some lines of a file |
| `expand.rs` | Showing more context lines around a hunk |
| `log.rs` | Commit history |
| `refs.rs` | Branches |
| `stash.rs` | Stashes |
| `merge.rs`, `progress.rs` | Merge / rebase / conflicts, "is an operation in progress?" |
| `history_ops.rs` | Revert, reset, squash, reword, reorder, tags, force push preview |
| `pr_preview.rs` | Comparing a branch with its base |
| `remote.rs`, `remotes.rs` | Parsing remote URLs; listing/editing remotes |
| `config.rs`, `identity.rs`, `info.rs` | Git config, user name/email, repository facts |
| `credentials.rs` | Handing account tokens to git network commands |
| `tests/` (crate root) | Integration tests that create real temporary repositories |

### `crates/forge` (gd-forge): CI web APIs

`client.rs` (one `Client` for every forge), `actions.rs` (GitHub + Gitea), `gitlab.rs`,
`forgejo.rs`, `model.rs` (the shared data types), `log.rs` (job log parsing: colors and
collapsible sections), `workflow.rs` (stages from a GitHub workflow file).

### `crates/ui` (gd-ui): look and feel

`theme.rs` (colors and sizes, `Theme::of(cx)`), `themes.rs` (loading JSON color themes),
`motion.rs` (animations), `assets.rs` (icons and fonts), `components/` (button, checkbox,
check_row, option_row, tabs, dialog, popover, context_menu, tooltip, toolbar_button,
resize_handle, surface).

### `crates/syntax` (gd-syntax): code colors

One file, `lib.rs`. Give it a file path and text, it returns colored ranges per line.
Supported languages are listed in `LanguageId`.

## Other top-level things

| Path | What |
|---|---|
| `vendor/gpui_ce`, `vendor/gpui_ce_parley` | Our patched copies of two GPUI crates (scroll speed, word wrapping). Wired in the root `Cargo.toml` `[patch]`. |
| `build-aux/flatpak/` | Flatpak packaging (Linux release) |
| `.github/workflows/` | Builds the Flatpak when you publish a GitHub release. Nothing else runs automatically. |
| `bump-version.sh` | Sets the version, commits and tags |
| `docs/THEMES.md`, `docs/theme.schema.json` | How to write a color theme |
| `.ref/` (git-ignored) | Read-only clones of reference apps, for reference |
| `.worktrees/` (git-ignored) | Old AI working copies; safe to delete when nothing is running in them |
