# desktop-rs — Rust + GPUI-CE rewrite

A native rewrite of [GitHub Desktop](https://github.com/desktop/desktop) in Rust, using [GPUI Community Edition](https://github.com/gpui-ce/gpui-ce)

It currently compiles and works for both Linux and macOS. I dont use windows and i also dont care about it so

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

### Headless screenshots

```sh
GD_CONFIG_DIR=/tmp/desktop-rs/config cargo run --features snapshot -- \
    --snapshot /tmp/desktop-rs/shot.png /path/to/repo [changes|history|repos|branches|new-branch|clone|welcome|icons]
```

This renders the real UI offscreen with wgpu and writes a PNG. It's how agents check visuals without a display.
