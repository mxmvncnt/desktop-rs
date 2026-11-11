# 2. Just enough Rust and GPUI to read this code

You don't need to learn all of Rust to change this app. You need to recognize about
fifteen patterns. They are listed here with a real example from the code and the Vue
idea that is closest.

## Rust in ten minutes

### Structs and `impl`

```rust
pub struct StashRow {          // like a component's props
    pub stash: Stash,
    pub selected: bool,
}

impl StashRow {                // methods of StashRow
    fn something(&self) { .. } // `&self` = read-only access to this row
}
```

`&self` means "I only read", `&mut self` means "I can change fields". If a function
takes `&mut self` and you see a compile error about "borrow", it usually means you're
trying to read and change the same thing at once. Clone the value you read first
(`let x = self.thing.clone();`) and the error goes away. That is why this code has
many `.clone()` calls.

### The same type can have `impl` blocks in many files

```rust
// crates/app/src/features/stashes/ops.rs
impl RepoModel {
    pub fn stash_pop(&mut self, ...) { .. }
}
```

`RepoModel` is defined in `repo/state.rs`, but each feature adds its own methods in its
own `ops.rs`. Same for `Workspace`: each feature adds methods in its `handlers.rs`.
So when you look for "where is `stash_pop` defined", **search the whole project**
(`rg "fn stash_pop"`), don't assume it's next to the struct.

### `Option` and `Result`

- `Option<T>` is "a T or nothing" (`Some(x)` or `None`). Like `T | null`.
- `Result<T, E>` is "a T or an error" (`Ok(x)` or `Err(e)`).
- `?` at the end of a call means "if this is an error/None, return it right away".
- `let Some(repo) = self.repo.clone() else { return; };` means "if there's no repo,
  stop here". You'll see this at the top of almost every handler.

### `match`

```rust
match action {
    StashAction::Apply => ...,
    StashAction::Pop => ...,
}
```

A `switch` that the compiler checks: if you add a new `StashAction` variant and forget
to handle it, **the build fails and points at every `match` you need to update**. Use
that: when you add an enum variant, build, and fix each error. That is the fastest way
to find every place a change must go.

### Closures

`move |this, cx| this.refresh(cx)` is an arrow function: `(this, cx) => this.refresh(cx)`.
`move` means it takes ownership of the variables it uses, which is why you'll see things
cloned just before a closure (`let repo = repo.clone(); move || repo.status()`).

### Modules (`mod.rs`)

Each folder has a `mod.rs` that lists its files (`mod stash_row;`) and says which names
other folders may use (`pub use stash_panel::StashPanel;`). A file that isn't listed in
`mod.rs` is not compiled at all. `pub(super)` means "visible to the parent folder only",
`pub(crate)` means "visible in this crate only".

### Derive

`#[derive(Clone, Default, Debug)]` auto-generates `.clone()`, `Default::default()` and
debug printing. `#[derive(Serialize, Deserialize)]` (from the `serde` crate) makes a
struct readable/writable as TOML or JSON. That is how `settings.toml` works.

## GPUI in ten minutes

### Entities = reactive stores

```rust
pub repo: Option<Entity<RepoModel>>,
```

An `Entity<T>` is a handle to a piece of state that GPUI owns. Think of a Pinia store
you hold a reference to.

- Read it: `repo.read(cx).branch_name()`
- Change it: `repo.update(cx, |r, cx| r.select_stash(sha, cx))`
- After changing, the store calls `cx.notify()`. That tells GPUI "anyone showing me must
  redraw". **If you change a field and forget `cx.notify()`, the screen won't update.**
  This is the number one bug you will hit.

`cx` ("context") is passed to almost every function. It's how you reach GPUI: read
entities, spawn tasks, open windows, write to the clipboard. Just pass it along.

### Two kinds of components

| GPUI | Vue | When |
|---|---|---|
| `#[derive(IntoElement)] struct X {..}` + `impl RenderOnce for X` | stateless SFC with props | Most components. Built, drawn, thrown away every frame. |
| `struct X {..}` + `impl Render for X`, used as `Entity<X>` | SFC with `data()` | Only when it keeps state between frames (a text input, the commit box). |

A `RenderOnce` component (from `features/stashes/stash_row.rs`, shortened):

```rust
#[derive(IntoElement)]
pub struct StashRow {
    pub stash: Stash,
    pub selected: bool,
    pub repo: Entity<RepoModel>,
    pub ws: Entity<Workspace>,
}

impl RenderOnce for StashRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        div()                                  // <div>
            .flex().flex_col()                 // class="flex flex-col"   (Tailwind-like)
            .h(px(46.0)).px(px(10.0))          // height 46px, padding-x 10px
            .text_color(theme.text)            // colors always come from the theme
            .child(self.stash.title())         // {{ stash.title }}
            .on_click(move |_, _, cx| {        // @click
                repo.update(cx, |r, cx| r.select_stash(sha, cx))
            })
    }
}
```

The method names (`flex`, `px`, `gap`, `rounded`, `text_size`, `border_1`...) are
Tailwind classes written as functions. If you know Tailwind, you can read any
template here.

You use it in a parent like `.child(StashRow { stash, selected, repo, ws })`.

### Events and callbacks

- `.on_click(move |event, window, cx| ..)`, `.on_mouse_down(MouseButton::Right, ..)`: DOM-like events.
- To call a method on the workspace from inside a component: `ws.update(cx, |this, cx| this.open_stash_menu(..))`.
  This is the "emit" of this code base. Almost every component receives `ws: Entity<Workspace>` for this reason.
- Some stores emit typed events (`cx.emit(RepoEvent::Fetched)`) and others listen with
  `cx.subscribe(&model, |this, _, event, cx| ..)`. Like `$emit` + `@event`.

### Background work (the most important pattern)

Git is slow. Running it on the UI thread freezes the window. So every git call looks
like this (from `features/stashes/ops.rs`):

```rust
pub fn reload_stashes(&mut self, cx: &mut Context<Self>) {
    let repo = self.repo.clone();                          // 1. copy what the background needs
    cx.spawn(async move |this, cx| {                       // 2. start an async task
        let res = cx
            .background_spawn(async move { repo.stash_list() })  // 3. run git on another thread
            .await;
        this.update(cx, |this, cx| {                       // 4. back on the UI thread
            if let Ok(list) = res {
                this.stashes.list = list;                  // 5. store the result
            }
            cx.notify();                                   // 6. redraw
        })
        .ok();                                             // 7. ignore "the model was closed meanwhile"
    })
    .detach();                                             // 8. let it run on its own
}
```

You will rarely write this by hand. There are two helpers that do it for you:

- `self.run(Op::default(), |repo| repo.something(), |this, result, cx| { .. }, cx)` for
  **write** operations (commit, stash, checkout...). It runs the git call in the
  background, puts errors in the banner, then refreshes everything. File: `repo/run.rs`.
- `self.load(|r| &mut r.some_slot, |repo| repo.something(), |this, result, cx| { .. }, cx)`
  for **reads** where only the latest result matters (you click file A then file B
  quickly: A's diff is thrown away). The first argument is a closure to a `LoadSlot`
  field — not the field itself — because `load` keeps the newest request id in it.

### Actions and keyboard shortcuts

```rust
actions!(desktop, [ShowChanges, Fetch, ...]);                 // crates/app/src/actions.rs
KeyBinding::new(&format!("{M}-shift-t"), Fetch, Some(WORKSPACE)) // M = ctrl/cmd
.on_action(cx.listener(|this, _: &Fetch, _, cx| ..))          // workspace/actions.rs: what it does
```

An action is a named command. A key binding maps a key to an action in a "key context"
(here `WORKSPACE`, a const in `actions.rs`). A handler on an element runs when the
action fires while that element (or something inside it) has focus.

### Lists that can be long

`uniform_list("id", count, |range, window, cx| rows_for(range))` only builds the rows
that are visible. Every list that could be long (files, commits, diff lines) must use
it, or scrolling a 50k-line diff will freeze. The rows of those lists are plain
functions, not components, for speed.

### Overlays (menus, dialogs, popovers)

Everything that floats above the window is an entry in `Workspace::overlays` (a
`Vec<Overlay>`, last = on top). Open one with `self.open_overlay(Overlay::X(..), cx)`,
close the top one with `self.close_overlay(window, cx)`. See
[03-how-it-works.md](03-how-it-works.md#menus-and-dialogs).

## Things that will confuse you

- **`cx.notify()` forgotten**: the field changed but the screen didn't.
- **Cached panes**: the sidebar and the main pane are "cached views"
  (`workspace/pane_view.rs`). They only redraw when the `Workspace` notifies. If you add
  a new store that a pane reads, make the workspace observe it:
  `cx.observe(&store, |_, _, cx| cx.notify())` in `workspace/init.rs`.
- **Animations inside the panes** can't use `with_animation` (it restarts on every
  redraw of a cached view). Use `motion::fade_quick_since(start_instant, ..)` and keep the
  `Instant` in state. Copy an existing one, e.g. `DiffView::shown`.
- **`+ use<>`** after `impl IntoElement` in a function signature is a Rust 2024 detail
  meaning "this doesn't borrow the arguments". If the compiler asks for it, add it.
- **The `snapshot` feature**: lots of code is behind `#[cfg(feature = "snapshot")]`. It
  only compiles with `cargo build --features snapshot`, used for headless screenshots.
  If you rename something, build with that feature too or you'll break it.
