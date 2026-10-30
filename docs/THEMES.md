# Custom themes

Themes are JSON files. Settings > Appearance > "Open themes folder" opens
`~/.config/desktop-rs/themes/`; drop a `.json` file there and restart the app.
A theme with the same `id` as a built-in replaces it. A file that fails to parse is
logged and skipped.

## Format

Copy one of the built-in themes in
[`crates/ui/assets/themes/`](../crates/ui/assets/themes) as a starting point
(`default.json` is the default). Each file has an `id`, a `label` and a `dark` and
a `light` palette of 19 `#rrggbb` colors. The System/Light/Dark mode picks the
variant, so both are required.

```json
{
  "$schema": "https://raw.githubusercontent.com/mxmvncnt/desktop-rs/main/docs/theme.schema.json",
  "id": "mine",
  "label": "Mine",
  "dark": { "bg": "#060606", "...": "19 colors in total" },
  "light": { "bg": "#ffffff", "...": "19 colors in total" }
}
```

## Editor support

The format is described by [`theme.schema.json`](theme.schema.json). Keep the
`"$schema"` line from the built-ins and editors will validate and complete your
theme. The app ignores that key.
