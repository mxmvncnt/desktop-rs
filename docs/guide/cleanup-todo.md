# Cleanup TODO: what to fix, in what order, with simple steps

This is the "vibe-coded fallout" list. It assumes you read
[01-what-is-this.md](01-what-is-this.md) first.

**What already happened.** An earlier cleanup (`docs/CLEANUP_REPORT.md`,
Oct 2026) removed the worst copies: 5 menu systems → 1 generic `Menu`,
4 file-diff flows → 1 `DiffDetail`, hand-written settings formats → `serde` +
TOML, hand-rolled watcher → `notify-debouncer-full`, and many dead helpers.
Don't re-do those. The list below is only what *remains* as of this writing.
Each item is a code smell: a pattern that makes the code harder to read,
duplicate, or force a small change to touch many files.

**How to use this file.** Work top to bottom. Each item says how big it is
and how risky. After each item run the daily loop from
[06-build-test-debug.md](06-build-test-debug.md):

```sh
cargo build && cargo build --features snapshot && cargo clippy --workspace --all-targets
```

If a step changes the look, also run the relevant `--snapshot` PNG and look.

---

## The map of the remaining problems

| # | Smell (plain words) | Where | Effort | Risk if you skip it |
|---|---|---|---|---|
| 1 | Two god objects hold everything | `workspace/state.rs`, `repo/state.rs` | Large | Every small change touches shared files; typing still re-renders more than needed |
| 2 | Adding a dialog touches 5 files | `workspace/overlay.rs`, `render.rs` | Medium | Dialogs get slower to add; bugs hide in the hand-written routing |
| 3 | Adding a list to arrow-key navigation touches 3 places | `workspace/navigation.rs` | Small | Lists get forgotten by ↑/↓ |
| 4 | A few files are too big to read in one go | `accounts/ops.rs` (499), `ci/ops.rs` (453), `highlight.rs` (322), `history_ops.rs` (475) | Small | You can't hold the feature in your head |
| 5 | Panicking on assumptions (`unwrap`/`expect` at runtime) | `commit_box.rs`, `snapshot.rs`, `main.rs` | Small | The app can crash instead of showing an error |
| 6 | The two biggest features (accounts, CI) barely talk to each other | `features/accounts/`, `features/ci/` | Small | The next contributor copies instead of reusing |
| 7 | macOS builds have never been compiled | `platform/macos/` | Medium | It may be broken right now and you'd only find out on a Mac |
| 8 | Vendor patches drift silently | `vendor/gpui_ce/`, `vendor/gpui_ce_parley/` | Small | A GPUI bump quietly loses our fixes |

---

## 1. Two god objects hold everything

**Where:** `crates/app/src/workspace/state.rs` (`Workspace`, 37 fields) and
`crates/app/src/repo/state.rs` (`RepoModel`, 18 fields grouped into
`work` / `log` / `sync` / `stashes` / `pr` / `merge` sub-structs). Measured
today (re-run these commands yourself after changes):

```sh
grep -rl "impl Workspace" crates/app/src --include="*.rs" | wc -l   # 39 files
grep -rl "impl RepoModel"  crates/app/src --include="*.rs" | wc -l   # 19 files
grep -rn "ws: Entity<Workspace>" crates/app/src --include="*.rs" | wc -l  # 52 fields
grep -rn "ws\.update(cx" crates/app/src --include="*.rs" | wc -l     # 63 call sites
```

So: `impl Workspace` methods live in **39 files** (every feature adds them in
its `handlers.rs`), **52 components** hold an `Entity<Workspace>` field, and
there are **63** `ws.update(cx, ..)` call sites. Almost every leaf component
depends on the root.

**Why it hurts (ELI5).** `Workspace` is like the one giant `App.vue` that
holds the state of every page. Any component can change any field, so
changing one thing (say, the stash dialog) can redraw things that have
nothing to do with it, and you must grep the whole project to learn what a
component touches. The Vue idea that never arrived: **one store per feature**
(Pinia), where a component only sees its own store.

**Simple steps.** Don't rewrite the app. Extract one store at a time,
starting with the one you touch most. `CiStore` (`features/ci/state.rs`) is
the model to copy: it is already an `Entity`, its methods take only
`Context<CiStore>`, and the only `Workspace` handle in that feature is
`CiInputs`' `WeakEntity<Workspace>` (for the filter inputs) plus events it
raises for the notice banner.

1. Pick a feature (start: `stashes` — small, well-isolated).
2. Turn its `*Ui` struct (`StashUi` in `features/stashes/state.rs`) into an
   `Entity<StashStore>` created in `workspace/init.rs`, like `AccountsStore`.
3. Change that feature's components from `ws: Entity<Workspace>` to
   `stash: Entity<StashStore>`. Change the call sites from
   `ws.update(cx, |this, cx| this.stash_xxx(..))` to
   `stash.update(cx, |s, cx| s.xxx(..))`.
4. In `workspace/init.rs`, add `cx.observe(&store, |_, _, cx| cx.notify())`
   so panes that read it still redraw.
5. Build, snapshot-test the feature, commit. Repeat for the next feature
   (`history`, `merge`, `branches`, …). Delete `Workspace` fields as you go;
   it should shrink on every commit.

**Do not confuse this with:** `HistoryUi` / `StashUi` etc. being "plain
structs on purpose". They are plain *because* the overlay routing in
`workspace/overlay.rs` reads their `menu`/`dialog` fields. Once dialogs are
generic (item 2), that excuse disappears.

## 2. Adding a dialog touches 5 files

**Where:** `workspace/overlay.rs` (the `Overlay` enum + `overlay_confirm`
match), `workspace/render.rs` (`overlays()` match), plus your feature's
`state.rs` / `handlers.rs` / view file.

**Why it hurts.** Menus were made generic (`Menu::new(position, rows, closure)`),
but dialogs still need a new enum variant + two `match` arms + a render arm
in the workspace files. Miss one arm and the compiler finds it — good — but
the knowledge is still in the wrong place, and every dialog feature depends
on the core files.

**Simple steps.**

1. Look at how `Menu` did it: the caller builds rows, the generic component
   owns `step()` / `confirm()` / rendering.
2. Give dialogs the same shape: an enum (or a `Box<dyn DialogSpec>`) in
   `gd_ui` whose caller supplies `content` (inputs), `on_confirm`, `on_cancel`.
   `Dialog::content` already does Enter/Esc for inputs — extend it.
3. Move each `*_confirm` (`stash_confirm`, `merge_confirm`, …) into the
   *callback* instead of a `match` in `overlay_confirm`.
4. Convert one dialog (stash) first, delete its `match` arms, snapshot-test,
   commit. Then the others. `Overlay` then keeps the shared kinds
   (`Popover`, `Menu`, `Dialog`, `DiffOptions`, `Settings`, `RepoSettings`,
   `ForcePush`) instead of one variant per dialog.

After this, [05-recipes.md](05-recipes.md#add-a-dialog) becomes "1 feature
folder + 1 callback" and item 1's stores can actually be extracted.

## 3. Adding a list to arrow-key navigation touches 3 places

**Where:** `workspace/navigation.rs` (`NavList` enum + `select_move` match).
Every navigable list wraps itself with `nav_focus(div, NavList::X, ws)`.

**Why it hurts.** A new list needs: a `NavList` variant, a `select_*`
method on `RepoModel`, and an arm in `select_move` that computes the target
and scrolls. The compiler catches a missing `select_move` arm (the `match` is
exhaustive), but not a missing `nav_focus(..)` on the list container — that
one silently makes ↑/↓ never target the list. The pain is the spread:
`select_move` (`workspace/navigation.rs`) is a hand-written `match` with
**four** arms (`Changes`, `Commits`, `Stashes`, `Files(DiffPane)`), each
re-doing "compute target, call `select_*`, `scroll_to_item`", plus a
`HistoryLog` search special case.

**Simple steps.**

1. Give the four lists a common shape: `trait NavigableList { fn len(&self)
   -> usize; fn selected(&self) -> Option<usize>; fn select(&mut self, ix,
   cx); fn scroll(&self, ix); }`.
2. Collapse the four arms of `select_move` into one call through the trait.
   Each list keeps a one-line `select_*` forwarding to it.
3. New lists then need *one* impl plus the `NavList` variant, not three
   places.

If this feels like it adds more than it removes, skip it — your
`docs/CONVENTIONS.md` says an abstraction must earn its place, and four arms
is not a crisis. Do it when you add the fifth list.

## 4. A few files are too big to read in one go

**Where (roughly 300+ lines):**

| File | Lines | Split it into |
|---|---|---|
| `crates/app/src/features/accounts/ops.rs` | 499 | `identity_ops.rs`, `provider_ops.rs` (by what the methods do) |
| `crates/git/src/history_ops.rs` | 475 | `rewrite.rs` (squash/reword/reset), `tags.rs`, `history_queries.rs` |
| `crates/app/src/features/ci/ops.rs` | 453 | `pipelines_ops.rs`, `jobs_ops.rs` |
| `crates/app/src/features/ci/snapshot.rs` | 420 | one `snapshot.rs` per sub-view (`pipelines_snapshot.rs`, `job_snapshot.rs`) |
| `crates/app/src/features/diff/highlight.rs` | 322 | `cache.rs` (the `HighlightCache` LRU) + the rest |
| `crates/syntax/src/lib.rs` | 560 | languages table (already noted in `docs/CLEANUP_REPORT.md` §4) |

Long functions are the same problem one level down. Measured with a
brace counter (nested code included, so a render fn that builds a big tree
counts as one long function):

| Function | Lines | File |
|---|---|---|
| `snapshot::run` | 217 | `crates/app/src/snapshot.rs` (the harness; leave it) |
| `PipelineView::render` | 209 | `features/ci/pipeline_view.rs` |
| `accounts_sample` | 158 | `features/accounts/snapshot.rs` (dev tool; leave it) |
| `Chrome::frame` | 132 | `platform/linux/chrome.rs` |
| `log_line` | 128 | `features/ci/log_view.rs` |
| `engine::diff_texts` | 106 | `crates/git/src/engine.rs` |
| `save_account` | 104 | `features/accounts/ops.rs` |
| `pipeline_row` | 106 | `features/ci/pipeline_row.rs` |

**Why it hurts.** You can't keep 500 lines of `impl AccountsStore` (or a
200-line render) in your head, so you copy instead of reusing, and the copies
*become* the duplication the old report cleaned up.

**Simple steps.** Each split is mechanical and safe:

1. Pick the file. Group its `fn`s by what they do (names already hint it).
2. Move a group to a new file in the same folder. Change `mod.rs` to list
   it. Visibility stays `pub(crate)` / `pub(super)` — `mod.rs` is the only
   file that usually changes.
3. Build + test that feature. Commit. One split per commit.

Rule of thumb for new code: a file that grows past ~350 lines gets split
before it grows further. Conventions say "aim < 40 lines per function" —
files need the same discipline.

## 5. Panicking on assumptions instead of showing an error

**Where:** 16 `.unwrap()` / `.expect()` outside tests. 10 are dev tools
(8 in `snapshot.rs`, 2 in `main.rs`) and can stay — they *should* fail loudly.
The rest:

| Location | What it assumes | Can it crash the app? |
|---|---|---|
| `features/changes/commit_box.rs:134` | a repo is open | yes (it is only rendered with a repo today, but nothing enforces it) |
| `features/history/handlers.rs:49` | the multi-selection is non-empty | yes, if the state gets out of sync |
| `frame_debug.rs:31` | the `LAST` mutex isn't poisoned | yes, but only when `GD_DEBUG_FRAMES` is on |
| `watcher.rs:125` | the debouncer slot mutex isn't poisoned | yes, on a watcher thread |
| `features/diff/highlight.rs:31` | `8` is non-zero | no — constant, can't fail |
| `ui/src/themes.rs:184` | the listed theme file is embedded | no — it is `Assets::get` on a file the same code just listed |

**Why it hurts.** A panic = the app dies. A `Result` = a red banner with a
message the user can act on. Dev-only panics (snapshot mode, window open) are
fine; runtime panics are not.

**Simple steps.**

1. Re-run the list:
   `grep -rn "\.unwrap()\|\.expect(" crates/*/src --include="*.rs" | grep -v "/tests/" | grep -v "\.test\.rs"`.
2. For each runtime one, replace with `let Some(x) = .. else { return; }`
   or `if let` / `match`. The `?` operator is your friend — but note `run`
   and `load` already turn git errors into banners, so a `?` inside an
   `ops.rs` closure usually does the right thing.
3. Leave snapshot/main panics; add a comment saying "dev tool, panics OK".
4. For mutexes (`frame_debug.rs`, `watcher.rs`): a poisoned mutex should
   log and continue, not panic. `if let Ok(mut guard) = LAST.lock()`.

## 6. Accounts and CI are the two biggest features but barely share code

**Where:** `features/accounts/` (accounts, tokens, OAuth, secrets — 2,773
lines) and `features/ci/` (pipelines, jobs, logs — 3,963 lines, including a
420-line snapshot file and 13 files over 100 lines).

**Why it hurts.** They are the most complex features and the easiest place
for the next copy to happen. Both need "show a list, load it in the
background, handle a failure, poll". Accounts renders a failure as a
`StatusText` (`features/accounts/status_text.rs`: `ok()` / `err()`); CI has
its own `Problem` enum + `ProblemView` (`features/ci/target.rs`,
`problem_view.rs`) and stores `error` / `list_error` / `detail_error` as
three separate fields. Those are two vocabularies for the same thing, and a
reader has to learn both before touching either feature.

**Simple steps.**

1. `account_form.rs` (387 lines) — split it into one file per section
   (identity fields / provider select / token / "own app"), reusing
   `components/field_row.rs` for the labelled inputs.
2. `log_view.rs` (386 lines) — extract the section header and the line
   rendering into their own files. `log_selection.rs` already shows the
   shape: selection is a small module next to the view.
3. Make CI's three error fields one `Problem` (or one `Option<Problem>` plus
   the list/detail distinction); don't force CI to use `StatusText` if
   `ProblemView` is better — the goal is one way to say "this failed", not
   one widget.
4. Each extraction is one commit with snapshot images before/after
   (they must be identical if you didn't change behaviour).

## 7. macOS has never been compiled

**Where:** `crates/app/src/platform/macos/` (window chrome and backdrop).
`docs/PROGRESS.md` says it was never compiled. The Linux code is
actively maintained; the macOS mirror is not.

**Why it hurts.** The platform layer is *designed* for symmetry: both
`platform/linux/` and `platform/macos/` expose `Chrome`, `caption_buttons`,
`titlebar_drag`, `frame`, `configure_window`, `Backdrop`, `SECONDARY`, … If
the two drift apart, macOS will break in ways nobody sees until a Mac user
tries it — and you won't know what "broken" looks like.

**Simple steps.**

1. Build it on a Mac: `cargo build` (there is no Mac here, and a Linux
   `cargo check --target aarch64-apple-darwin` is not trustworthy — it hits
   Apple frameworks and C dependencies long before it checks our code).
2. Diff the two `platform/*/mod.rs` surfaces. They share every name the rest
   of the app uses (`Backdrop`, `Chrome`, `CAPTION_SIZE`, `caption_buttons`,
   `titlebar_drag`, `frame`, `configure_window`, `SECONDARY`,
   `SECONDARY_LABEL`, `REFRESH_KEY`, `QUIT_KEY`); Linux additionally exports
   its corner/shadow constants. Keep the shared surface identical.
3. Compare the two `backdrop.rs` files line by line — the "Window
   background" section of `docs/ARCHITECTURE.md` describes both behaviours.
4. If you find a macOS-only bug, fix it and document it in
   `docs/ARCHITECTURE.md` ("Platform code" section). If you can't compile
   it, say so in `docs/guide/01-what-is-this.md` "What works and what
   doesn't" so the next person knows.

## 8. Vendor patches drift silently

**Where:** root `Cargo.toml` `[patch]` — `vendor/gpui_ce` (scroll fixes) and
`vendor/gpui_ce_parley` (word wrapping). Plus the pinned GPUI-CE rev in
`Cargo.toml`.

**Why it hurts.** Two silent failure modes: (a) someone bumps the GPUI-CE
pin and forgets the vendored crates, losing the patches; (b) someone edits
`vendor/` thinking it's upstream and the patch is never documented. The
parley one is *one line* (`OverflowWrap::Anywhere` in `text_system.rs`) —
easy to lose, hard to notice.

**Simple steps.**

1. When you bump the pin: re-apply both patches and re-run
   `GD_SNAPSHOT_BENCH` (scroll) and a long-word diff snapshot (parley).
   The benches are the actual test for the vendor patches.
2. Never edit `vendor/` without a comment at the top of the file saying
   what the patch is and why. Check the two vendored crates for such
   headers today and add them if missing.
3. Keep the pin section of `docs/ARCHITECTURE.md` (§GPUI-CE pin) updated.
   It is the checklist for a bump.

---

## What is *not* on this list, and why

- **"Turn everything into per-feature stores"** (old item 3) — partially
  done (`CiStore`, `AccountsStore`, `RepoSettingsStore`). The remaining
  `*Ui` structs are *data*, not stores, on purpose (see
  `docs/CONVENTIONS.md` "Stores"). Items 1–2 above are the honest remainder.
- **The old `docs/CLEANUP_REPORT.md` Parts 1–2** — done. Don't re-read it as
  a TODO list; it is history.
- **No `TODO`/`FIXME` markers in the code** (grep returns nothing) and
  `cargo clippy` is clean apart from one upstream warning. The smells that
  remain are structural, not linter-visible — that's why this file exists.
- **Performance:** the app is already careful (virtualized lists, background
  git, no idle animations, `GD_SNAPSHOT_BENCH` numbers in
  `docs/ARCHITECTURE.md`). After any change to a diff row or list, re-run
  the bench. If it drops, you have changed the perf, not just the code.

## Suggested order (each step leaves the app working)

1. Item 5 (panics) + Item 4 for one file — the warm-up, one sitting.
2. Item 3 (navigation trait) — small, self-contained, proves the pattern.
3. Item 2 (generic dialogs) — unlocks item 1.
4. Item 1 (stores, one feature per commit, starting with stashes).
5. Items 6, 7, 8 as you touch those areas.
