//! Embedded assets: Geist fonts (OFL), Solar icons (CC BY 4.0,
//! "Solar Icons by 480 Design") and the built-in color theme JSON files, copied from zeron's `crates/ui/assets`.
//!
//! `rust-embed` bakes `assets/` into the binary (also in debug builds), so
//! icon loads never touch the filesystem. Adding an icon = drop the SVG in
//! `assets/icons/` and add one line to the `icons!` list.

use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

macro_rules! icons {
    ($($name:ident => $file:literal),+ $(,)?) => {
        /// Icon names, for `icon(Icon::Check)` / `svg().path(...)`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Icon { $($name),+ }

        impl Icon {
            pub fn path(self) -> &'static str {
                match self { $(Self::$name => concat!("icons/", $file, ".svg")),+ }
            }
        }
    };
}

icons! {
    Archive => "archive-minimalistic",
    ArchiveUp => "archive-up-minimalistic",
    ChevronDown => "alt-arrow-down",
    ChevronRight => "alt-arrow-right",
    ChevronUp => "alt-arrow-up",
    ArrowDown => "arrow-down",
    ArrowUp => "arrow-up",
    ArrowLeft => "arrow-left",
    ArrowRight => "arrow-right",
    Check => "check",
    Close => "close",
    CloseCircle => "close-circle",
    Clock => "clock-circle",
    Cloud => "cloud",
    Copy => "copy",
    Document => "document",
    DocumentAdd => "document-add",
    Eye => "eye",
    EyeClosed => "eye-closed",
    Folder => "folder",
    FolderWithFiles => "folder-with-files",
    GitBranch => "git-branch",
    Globe => "global",
    Info => "info-circle",
    Search => "magnifer",
    More => "more-horizontal",
    Plus => "plus",
    PullRequest => "pull-request",
    Refresh => "refresh",
    Restart => "restart",
    Settings => "settings",
    Sort => "sort",
    SortVertical => "sort-vertical",
    Tag => "tag",
    Terminal => "terminal",
    Trash => "trash-bin-minimalistic",
    List => "list",
    Warning => "danger-triangle",
    Fork => "fork",
    Pen => "pen",
    WindowMinimize => "window-minimize",
    WindowMaximize => "window-maximize",
    WindowRestore => "window-restore",
    Tuning => "tuning",
}

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
#[include = "fonts/*.ttf"]
#[include = "themes/*.json"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|f| f.data))
    }

    /// GPUI-CE builds its `AssetRegistry` by enumerating `list("")` once at startup.
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter().filter(|p| p.starts_with(path)).map(|p| SharedString::from(p.into_owned())).collect())
    }
}

pub fn load_fonts(cx: &App) {
    let fonts = Assets::iter().filter(|p| p.starts_with("fonts/")).filter_map(|p| Assets::get(&p)).map(|f| f.data).collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        log::error!("failed to load embedded fonts: {e:#}");
    }
}
