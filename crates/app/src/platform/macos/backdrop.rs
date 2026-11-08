//! Transparent and blurred backgrounds through gpui's native support
//! (an `NSVisualEffectView` behind the content for Blurred).

use gpui::{Window, WindowBackgroundAppearance};

use super::Chrome;

#[derive(Default)]
pub struct Backdrop(Option<WindowBackgroundAppearance>);

impl Backdrop {
    pub fn new(_window: &Window) -> Self {
        Self::default()
    }

    pub fn blur_supported(&self) -> bool {
        true
    }

    /// Called on every workspace render; only a change reaches the window.
    pub fn update(
        &mut self,
        appearance: WindowBackgroundAppearance,
        _chrome: Chrome,
        window: &Window,
    ) {
        if self.0 != Some(appearance) {
            self.0 = Some(appearance);
            window.set_background_appearance(appearance);
        }
    }
}
