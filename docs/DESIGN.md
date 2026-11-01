# Design: GitHub Desktop's layout, zeron's look

**Layout** stays Desktop's, so users find everything where they expect it:

- Toolbar: `Current repository ▾` | `Current branch ▾` | contextual remote button
  (Publish branch, Pull n, Push n, or Fetch).
- Sidebar (300px): segmented `Changes (n) | History` tabs.
  - Changes: an "all files" checkbox, then a file list (checkbox, dim dir, bright name,
    colored status glyph), then the commit box (one growing message box,
    `Commit to <branch>` primary button).
  - History: a commit list (summary over `author • relative time`).
- Main: the diff for the selected file. For History, a commit header
  (summary, body, author, sha, time) above a file list (260px) and the diff.
- No changes: a centered "No local changes" with suggestion cards (push, pull, open folder).
- No repo: "Let's get started!" with Add and Clone cards.

**Look** comes from zeron (`.ref/zeron/crates/ui/src/theme.rs`, `motion.rs`;
screenshots in `.ref/zeron/docs/screenshots/`):

- A near-black elevation ladder: `bg` #060606 (diff plane), `surface` #0d0d0d
  (toolbar, sidebar), overlay #161616 (popovers).
- No opaque greys for states. Hover and selection are translucent white "ink"
  washes (`element_hover` 7%, `element_active` 11%), and they **transition** (120ms), never snap.
- Hairline borders at 8% white. Rounded 6px controls, 10–12px panels.
- One primary action per view: a near-white solid plate with a dark label (`solid`/`on_solid`).
- Type: Geist (UI) and Geist Mono (diff, sha). Sizes 11–13px, muted captions over medium titles.
- Motion: entrances use `fade_in` (500ms, ease-out-quint, 4px rise). Popovers use
  `menu_in` (140ms, −2px). Content swaps use `fade_quick` (150ms).
  All honor `reduce_motion`.
- Controls say what they do and what state they are in:
  - A setting that is on or off is a checkbox row (`CheckRow`) with a fixed label that names what is on ("Current branch only"). Never a button.
  - One choice among a few is a radio group (`OptionRow`) or segmented `Tabs`; among many, a dropdown.
  - A button is an action: its label says what happens when it is pressed. No toggle buttons, and never a label that flips with the state
    ("Show x" / "Hide x", "All branches" / "This branch"): it does not tell the current state or what the next click does.
  - Exception: an icon-only toolbar button that opens or closes a page or panel (CI, settings) may show a pressed state while that is open.
- Status colors: emerald add, red delete, amber modified, blue renamed, at 10% washes in diffs.

Color themes are JSON files (`crates/ui/assets/themes/<id>.json`); see THEMES.md for the format and
ARCHITECTURE.md, "Color themes", for the implementation.

To add a component, put it in `crates/ui/src/components.rs`, read colors only
from `Theme`, give hover states a `.transitions(..)`, and check it with a `--snapshot`.
