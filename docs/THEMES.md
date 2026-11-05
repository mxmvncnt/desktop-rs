# Custom themes

Themes are JSON files. Settings > Appearance > "Open themes folder" opens
`~/.config/desktop-rs/themes/`; drop a `.json` file there and restart the app.
A theme with the same `id` as a built-in replaces it. A file that fails to parse is
logged and skipped.

## Format

Copy one of the built-in themes in
[`crates/ui/assets/themes/`](../crates/ui/assets/themes) as a starting point
(`default.json` is the default). Each file has an `id`, a `label` and a `dark` and
a `light` palette of `#rrggbb` colors. Syntax highlighting colors are fixed (one dark and one light set) and not themeable; the old `comment`, `keyword`, `string`, `number` and `special` keys are ignored. The System/Light/Dark mode picks the
variant, so both are required.

```json
{
  "$schema": "https://raw.githubusercontent.com/mxmvncnt/desktop-rs/main/docs/theme.schema.json",
  "id": "mine",
  "label": "Mine",
  "dark": { "bg": "#060606", "...": "more colors" },
  "light": { "bg": "#ffffff", "...": "more colors" }
}
```

## Diff colors

By default diff colors are derived from `success`, `danger`, `accent` and `faint`.
A palette can override them with these optional `#rrggbb` keys (see the schema for
descriptions):

- Line backgrounds: `diff_add_bg`, `diff_del_bg`, `diff_hunk_bg`
- Word-level emphasis: `diff_add_emph`, `diff_del_emph`
- Line-number cell backgrounds: `diff_add_gutter_bg`, `diff_del_gutter_bg` (default to the
  matching line background), `diff_hunk_gutter_bg` (hunk header gutter and expand buttons),
  `diff_gutter_bg` (unchanged lines)
- `diff_empty_bg`: split view placeholder opposite an added or deleted line
- `diff_gutter`: line number text (default `faint`)

## Surface opacity

When Settings > Appearance > Window background is Transparent or Blurred, the desktop shows
through the app. A palette sets how much with `opacity`, from 0 (invisible) to 1 (opaque), per
surface rather than per color. Every key is optional; these are the defaults:

```json
"opacity": { "bg": 0.8, "surface": 0.7, "card": 0.85, "overlay": 0.95, "raised": 0.9 }
```

- `bg`: diff plane, welcome screen and CI page
- `surface`: toolbar and sidebar
- `card`: cards and panels
- `overlay`: popovers, menus, dialogs and tooltips
- `raised`: chips and pills

Opaque ignores these values. Hover washes, diff colors and other tints keep their own alpha.

## Commit type pills

Commit titles that follow the conventional commit format show their type (`feat`, `fix`, ...)
in a pill tinted with the type's color. Defaults: `feat` success, `fix` danger, `docs` accent,
`perf` warning, `refactor`, `style` and `test` fixed purple, pink and amber. Types without a color
get a neutral pill. A palette can add or override colors with `commit_types`:

```json
"commit_types": { "feat": "#00d492", "chore": "#a1a1a1" }
```

## Editor support

The format is described by [`theme.schema.json`](theme.schema.json). Keep the
`"$schema"` line from the built-ins and editors will validate and complete your
theme. The app ignores that key.
