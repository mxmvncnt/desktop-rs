# desktop-rs — Rust + GPUI-CE rewrite

A native rewrite of [GitHub Desktop](https://github.com/desktop/desktop) in Rust, using [GPUI Community Edition](https://github.com/gpui-ce/gpui-ce)

## Screenshots

<img width="1280" height="820" alt="Screenshot From 2026-10-25 22-26-22" src="https://github.com/user-attachments/assets/668a8f85-6893-4ce5-a301-8d1d8b6ef38e" />

<img width="1280" height="820" alt="Screenshot From 2026-10-25 22-26-33" src="https://github.com/user-attachments/assets/9b4a0c94-483b-4937-9a82-e8c0e3b91abf" />

<img width="1280" height="820" alt="Screenshot From 2026-10-25 22-27-00" src="https://github.com/user-attachments/assets/50ff8165-c730-498a-9509-61f1eab41756" />

## Build and run

```sh
cargo run
```

The app stores its repository list and settings in `~/.config/desktop-rs/`.

### Keyboard shortcuts

| Keys            | Action                                    |
| --------------- | ----------------------------------------- |
| Ctrl+1 / Ctrl+2 | Changes / History tab                     |
| Ctrl+T          | Repository list                           |
| Ctrl+B          | Branch list                               |
| Ctrl+Shift+N    | New branch                                |
| Ctrl+O          | Add existing repository                   |
| Ctrl+Shift+T    | Fetch                                     |
| Ctrl+Shift+P    | Pull                                      |
| Ctrl+P          | Push / publish                            |
| Ctrl+Enter      | Commit                                    |
| F5              | Refresh                                   |
| ↑ / ↓           | Move through file, commit or picker lists |
| Esc             | Close popover                             |
| Ctrl+Q          | Quit                                      |

### Custom themes

Themes are JSON files you can add or override. See [docs/THEMES.md](docs/THEMES.md).

### Headless screenshots

```sh
GD_CONFIG_DIR=/tmp/desktop-rs/config cargo run --features snapshot -- \
    --snapshot /tmp/desktop-rs/shot.png /path/to/repo [changes|history|repos|branches|new-branch|clone|welcome|icons]
```

This renders the real UI offscreen with wgpu and writes a PNG. It's how agents check visuals without a display.
