//! Workspace-side settings handlers: open/close the modal, switch page,
//! apply the theme.

use gd_ui::Theme;
use gd_ui::themes::ColorThemes;
use gpui::{Context, Window, WindowBackgroundAppearance};

use super::{SettingsPage, ThemeMode, WindowBackground};
use crate::workspace::{Overlay, Workspace};

impl Workspace {
    pub fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        if matches!(self.overlays.last(), Some(Overlay::Settings)) {
            self.overlays.pop();
            return cx.notify();
        }
        self.open_overlay(Overlay::Settings, cx);
        if self.settings_page == SettingsPage::Accounts {
            self.load_identity(cx);
        }
    }

    /// Settings on the Accounts page (the CI page's "Change" and "Add an account").
    pub fn open_accounts_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_page = SettingsPage::Accounts;
        self.open_overlay(Overlay::Settings, cx);
        self.load_identity(cx);
    }

    fn load_identity(&mut self, cx: &mut Context<Self>) {
        self.accounts.update(cx, |a, cx| a.load_identity(None, cx));
    }

    pub(super) fn set_settings_page(&mut self, page: SettingsPage, cx: &mut Context<Self>) {
        self.settings_page = page;
        if page == SettingsPage::Accounts {
            self.load_identity(cx);
        }
        cx.notify();
    }

    /// Resolve the theme setting (System follows the window appearance) and install it.
    pub fn apply_theme(&mut self, window: &Window, cx: &mut Context<Self>) {
        use gpui::WindowAppearance as A;
        let system_light = matches!(window.appearance(), A::Light | A::VibrantLight);
        let light = self.settings.theme.is_light(system_light);
        let translucent = self.background_appearance() != WindowBackgroundAppearance::Opaque;
        let id = self.settings.color_theme_id(ColorThemes::of(cx));
        let theme = Theme::for_family(id, light, translucent, cx);
        cx.set_global(theme);
        cx.notify();
    }

    pub(super) fn set_color_theme(&mut self, id: &str, window: &Window, cx: &mut Context<Self>) {
        self.settings.color_theme = id.to_string();
        self.settings.save();
        self.apply_theme(window, cx);
    }

    /// The window background in effect: Blurred falls back to Transparent where blur is unavailable.
    pub fn background_appearance(&self) -> WindowBackgroundAppearance {
        match self.settings.window_background {
            WindowBackground::Opaque => WindowBackgroundAppearance::Opaque,
            WindowBackground::Blurred if self.backdrop.blur_supported() => {
                WindowBackgroundAppearance::Blurred
            }
            _ => WindowBackgroundAppearance::Transparent,
        }
    }

    pub(super) fn set_window_background(
        &mut self,
        background: WindowBackground,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.settings.window_background = background;
        self.background_menu = false;
        self.settings.save();
        self.apply_theme(window, cx);
    }

    pub(super) fn toggle_background_menu(&mut self, cx: &mut Context<Self>) {
        self.background_menu = !self.background_menu;
        cx.notify();
    }

    pub(super) fn set_theme_mode(
        &mut self,
        mode: ThemeMode,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.settings.theme = mode;
        self.settings.save();
        self.apply_theme(window, cx);
    }
}
