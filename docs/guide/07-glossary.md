# 7. Glossary: the words used in this code

Two halves. **Git words** are standard git; you can also learn them in the git
book. **App words** are desktop-rs's own names — learn these on day one and
don't invent synonyms.

## Git words (what git calls things)

| Word | What it means here | Where in the code |
|---|---|---|
| repository / repo | A folder with a `.git` inside. The app's `Repository` (`crates/git/src/lib.rs`) is a *handle* to it; opening is cheap, nothing is cached between calls. | `crates/git/src/lib.rs` |
| working tree | Your files on disk, as you just saved them. | `WorkingStatus` in `crates/git/src/status.rs` |
| index / staged | The "next commit" list. Checking a file in the UI puts it in the index. | partial staging: `crates/git/src/partial.rs` |
| HEAD | "Where am I": the commit the current branch points to. | `amend_commit`, `HEAD` strings in `crates/git/src/history_ops.rs` |
| branch | A movable name pointing to a commit. `HEAD` names the current one. | `Branch` in `crates/git/src/refs.rs` |
| upstream | The remote branch this branch tracks (for Pull/Push). | `% (upstream:track)` in `crates/git/src/refs.rs` |
| ahead / behind | Commits you have that the remote doesn't / vice versa. "Diverged" = both. | `AheadBehind` in `crates/git/src/status.rs` |
| commit | A saved snapshot with a message, author and parents. | `Commit` in `crates/git/src/log.rs` |
| diff | The difference between two versions of a file. The main diff view is computed by the app itself (`engine.rs`, `similar` crate), not by `git diff`; a few summaries/stash checks still shell out to `git diff --name-status`. | `Diff`, `crates/git/src/engine.rs` |
| hunk | One contiguous changed block in a diff. | `DiffHunk` in `crates/git/src/diff.rs`, expansion in `crates/git/src/expand.rs` |
| stash | A parked set of changes you can re-apply later. Identified by commit sha, never by index. | `Stash` in `crates/git/src/stash.rs` |
| merge / rebase / cherry-pick / revert | Ways to bring commits together or re-apply them. Each can *pause* on conflicts. | `crates/git/src/merge.rs`, `progress.rs` (`InProgress`) |
| conflict | Both sides changed the same lines. You resolve by editing the file and marking it resolved. | `crates/git/src/progress.rs`, `features/merge/conflict_*` |
| force push | Rewriting the remote branch to match your rewritten history. Dangerous; always previewed first. | `crates/git/src/history_ops.rs` (`force_push_preview`), `features/force_push/` |
| remote | A stored URL (GitHub/GitLab/…) you fetch/push with. | `Remote` in `crates/git/src/remotes.rs` |
| fetch / pull / push | Download / download+merge / upload. | `features/toolbar/ops.rs` |
| tag | A permanent name for one commit (releases). | `history_ops.rs` (`tags_by_commit`) |
| SHA / sha | The unique id of a commit (`a1b2c3d…`). Short form is shown in the UI. | everywhere |
| `porcelain` | Git's "machine-readable output" mode. All our parsing uses it (`status --porcelain=v2`, `-z`). | parsers in `status.rs`, `log.rs`, `refs.rs`; env pinned in `exec.rs` |
| `.gitignore` / `info/exclude` | Files to ignore. The watcher respects them. | `crates/app/src/watcher.rs` |

## App words (desktop-rs's own names)

| Word | What it means | Defined in |
|---|---|---|
| **crate** | A Rust package (≈ npm package). Five crates: `git`, `forge`, `syntax`, `ui`, `app`. | root `Cargo.toml` |
| **Workspace** | The whole window: open menus/dialogs, text inputs, scroll handles, settings. The root component. | `crates/app/src/workspace/state.rs` |
| **RepoModel** | The open repository's data: status, commits, branches, selected diff. The main store. | `crates/app/src/repo/state.rs` |
| **Entity\<T\>** | A handle GPUI owns for a piece of state, ≈ a Pinia store reference. `repo.update(cx, \|r, cx\| ..)` changes it. | GPUI |
| **cx / window** | `cx` = context (reaches GPUI: read entities, spawn tasks, clipboard). Passed to almost every function. | GPUI |
| **cx.notify()** | "I changed, redraw". Forget it and the screen never updates — the #1 bug. | — |
| **RenderOnce / Render** | `RenderOnce` = stateless SFC (props + `render()`); `Render` = SFC with `data()` (keeps state). | `crates/app/src/features/*` |
| **`impl Workspace` / `impl RepoModel`** | Methods on the big objects, *scattered across feature folders* in `handlers.rs` / `ops.rs`. Search the whole project. | — |
| **feature** | One folder in `features/`: `stash_row.rs` + `ops.rs` + `handlers.rs` (+ `state.rs`, `snapshot.rs`). | `crates/app/src/features/<name>/` |
| **component** | One visual piece in one file, used by the app (`components/`) or the design system (`crates/ui/src/components/`). | — |
| **store** | An `Entity` that owns state with logic: `RepoModel`, `PopoverStore`, `AccountsStore`, `CiStore`, `RepoSettingsStore`. | `features/*/state.rs` |
| **overlay** | Anything floating: a `Menu`, a dialog, a popover, the settings modal. All in one list `Workspace::overlays`; last = on top. | `crates/app/src/workspace/overlay.rs` |
| **picker** | A toolbar dropdown (repository list, branch list, new branch, clone). | `features/pickers/` |
| **DiffDetail** | "File list + selected file + diff + colors" for *every* place a diff shows (Changes, History, Stash, PR preview, force push). One struct, shared. | `crates/app/src/repo/diff_detail.rs` |
| **DiffPane** | Which `DiffDetail` you mean: `Changes`, `Commit`, `Stash`, `Pr`, `ForcePush`. | `crates/app/src/features/diff/state.rs` |
| **LoadSlot** | "Only the latest result counts" guard for background reads (click A then B fast → A's diff discarded). | `crates/app/src/repo/run.rs` |
| **run / load** | The two background helpers: `run` = git write ops, `load` = git reads where latest wins. | `repo/run.rs` |
| **refresh** | Re-reading the repo from git (status, branches, history…). `Status` = worktree only, `Full` = also git metadata. | `repo/refresh.rs` |
| **snapshot (view)** | A headless screenshot setup in `snapshot.rs` per feature: `View` + `steps` that set state, rendered by `cargo run --features snapshot`. | `crates/app/src/snapshot.rs` |
| **forge** | A GitHub/GitLab/Gitea/Forgejo web API (CI page). `Client` + `Forge` enum in `crates/forge/`. | `crates/forge/src/client.rs` |
| **pipeline / job / step** | CI words: a pipeline run, one job inside it, steps inside a job (GitHub/Gitea only). | `crates/forge/src/model.rs` |
| **theme / palette** | `Theme` = the live token struct (`Theme::of(cx)`); `Palette` = the raw colors a JSON theme file provides. | `crates/ui/src/theme.rs`, `themes.rs` |
| **GPUI-CE** | The GPUI framework (Zed's UI engine), community fork `gpui-ce`. Vendored patches in `vendor/`. | root `Cargo.toml` |
| **zeron** | The app whose design system `gd-ui` copies (components, motion, syntax palette). It is not in this checkout; only its look survives here. | — |
| **partial staging** | Committing only *some lines* of a changed file (drag in the gutter). | `crates/git/src/partial.rs`, `features/diff/gutter/` |
