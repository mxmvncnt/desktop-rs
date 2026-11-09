# 1. What is this?

desktop-rs is a git app that looks and behaves like
[GitHub Desktop](https://github.com/desktop/desktop), rewritten in Rust so it starts
fast and uses little memory (around 65 MB). It draws its own UI with **GPUI**, the UI
framework of the Zed editor (we use the community fork, "GPUI-CE"). The visual style
comes from another app called **zeron**.

The old Electron version is not in the working tree. `docs/AGENTS.md` says it
lives in git history on a `development` branch, but **this clone has no such
branch** (`git branch -a | grep development` returns nothing), so don't count
on it. If you ever fetch it, the old screens are the reference for "how did
GitHub Desktop do this?":

```sh
git show development:app/src/ui/changes/changes-list.tsx
```

## The five crates (= five packages)

A Rust "crate" is like an npm package. This repo is a workspace with five of them,
in `crates/`:

```
crates/
├── git/     gd-git     Talks to git. No UI code at all.
├── forge/   gd-forge   Talks to GitHub / GitLab / Gitea / Forgejo web APIs (for the CI page).
├── syntax/  gd-syntax  Colors code in diffs (syntax highlighting).
├── ui/      gd-ui      Look and feel: colors, fonts, icons, animations, basic widgets (button, checkbox, dialog...).
└── app/     desktop-rs The application itself: windows, screens, features. Uses the four others.
```

Only `app` knows about all four. The helper crates barely know each other: `forge`
borrows the remote-URL parser from `git`, and `ui` uses the list of highlight kinds
from `syntax` to pick their colors. Neither of them has any UI-framework code except
`ui`. This is the
cleanest part of the design: if you break something in `git`, the tests in
`crates/git/tests/` will tell you without starting the UI.

## How the pieces talk

```
            you click / type
                  │
                  ▼
   ┌──────────────────────────────┐
   │ Workspace  (the window)      │  open menus/dialogs, inputs, scroll, settings
   │   crates/app/src/workspace/  │
   └──────────────┬───────────────┘
                  │ "do this to the repo"
                  ▼
   ┌──────────────────────────────┐
   │ RepoModel  (open repository) │  status, commits, branches, selected diff
   │   crates/app/src/repo/       │
   └──────────────┬───────────────┘
                  │ runs on a background thread
                  ▼
   ┌──────────────────────────────┐
   │ gd-git  →  the `git` program │  `git status`, `git commit`, ...
   └──────────────────────────────┘
                  │ result comes back
                  ▼
     RepoModel stores it, says "I changed" (cx.notify())
                  │
                  ▼
     the screen redraws with the new data
```

There is also a **file watcher** (`crates/app/src/watcher.rs`). When you save a
file in your editor, it notices and asks `RepoModel` to refresh, which is why the
file list updates by itself.

## The features

Each feature has a folder in `crates/app/src/features/`:

| Folder | What it is on screen |
|---|---|
| `toolbar` | The top bar: repository button, branch button, Fetch/Pull/Push button |
| `pickers` | The dropdowns that open from the toolbar: repository list, branch list, new branch, clone |
| `changes` | The "Changes" tab: file list with checkboxes, commit message box, undo strip, sidebar |
| `diff` | The diff viewer (used by changes, history, stashes, PR preview, force push) |
| `history` | The "History" tab: commit list, commit details, right-click actions (revert, squash, ...) |
| `stashes` | The stash list under the file list, and the stash dialogs |
| `branches` | Branch right-click menu, rename / delete / create-from dialogs |
| `merge` | Merge / squash / rebase dialog, the "merge in progress" banner, conflict menu |
| `menus` | Right-click menu on changed files, and the "discard changes" dialog |
| `pr_preview` | "Preview Pull Request" screen |
| `force_push` | The "are you sure you want to force push?" preview |
| `ci` | The CI pipelines page (GitHub Actions, GitLab CI, ...) |
| `settings` | The settings window, and the saved `settings.toml` |
| `accounts` | Settings → Accounts: your git name/email, GitHub/GitLab sign-in, tokens |
| `repo_settings` | The per-repository settings window: remotes, .gitignore, git config |
| `notices` | The error / success banner at the bottom |
| `welcome` | The "Let's get started" screen when no repository is open |

## Where your data lives

- Settings and the repository list: `~/.config/desktop-rs/settings.toml`.
- Accounts (without tokens): `~/.config/desktop-rs/accounts.toml`.
- Which account each repository uses: `~/.config/desktop-rs/repo_accounts.toml`.
- Tokens: in the system keyring (GNOME Keyring / KWallet on Linux, Keychain on macOS). If
  there is no keyring, they go to `~/.config/desktop-rs/secrets.toml` in plain text and
  the Accounts page shows a warning.
- Custom color themes: `~/.config/desktop-rs/themes/*.json` (see `docs/THEMES.md`).

Set `GD_CONFIG_DIR=/some/folder` to use another folder. Do that while testing so you
don't mess up your real settings.

## What works and what doesn't

- Linux is the main platform and works.
- macOS code exists (`crates/app/src/platform/macos/`) but **has never been compiled**.
- Windows has no platform code at all.
- Many interactive things (right-click menus, drag, clipboard) were only checked through
  screenshots, never by a person with a mouse. `docs/PROGRESS.md` lists what was never
  tried by hand. If something feels broken, it may never have worked.
