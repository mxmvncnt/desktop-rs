//! Background blur on Wayland through `ext_background_effect_manager_v1`. gpui-ce
//! only speaks KDE's blur protocol, so we bind the global on our own event queue
//! over gpui's `wl_display` and attach the effect to gpui's `wl_surface`. The
//! blur region is double-buffered: gpui's next frame commits it. X11 and
//! compositors without the global (or without its blur capability) get none.

use gpui::{Window, WindowBackgroundAppearance};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::protocol::{wl_compositor, wl_region, wl_registry, wl_surface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1 as manager, ext_background_effect_surface_v1 as effect,
};

use super::Chrome;

pub struct Backdrop(Option<Wayland>);

struct Wayland {
    queue: EventQueue<Globals>,
    globals: Globals,
    surface: wl_surface::WlSurface,
    effect: Option<effect::ExtBackgroundEffectSurfaceV1>,
    /// Blur rectangle and corner radius last sent (`None`: no blur).
    sent: Option<(i32, i32, i32, i32, i32)>,
}

#[derive(Default)]
struct Globals {
    compositor: Option<wl_compositor::WlCompositor>,
    manager: Option<manager::ExtBackgroundEffectManagerV1>,
    can_blur: bool,
}

impl Backdrop {
    pub fn new(window: &Window) -> Self {
        Self(
            Wayland::connect(window)
                .inspect_err(|e| log::info!("no background blur: {e}"))
                .ok(),
        )
    }

    pub fn blur_supported(&self) -> bool {
        self.0.as_ref().is_some_and(|w| w.globals.can_blur)
    }

    /// Called on every workspace render; talks to the compositor only when the blur region changes.
    /// The window itself stays transparent (the CSD frame needs it), whatever the setting.
    pub fn update(
        &mut self,
        appearance: WindowBackgroundAppearance,
        chrome: Chrome,
        window: &Window,
    ) {
        let Some(w) = &mut self.0 else { return };
        // Capability changes; gpui's reads route our events to this queue.
        let _ = w.queue.dispatch_pending(&mut w.globals);
        let size = window.viewport_size();
        let inset = chrome.inset() as i32;
        let want =
            (appearance == WindowBackgroundAppearance::Blurred && w.globals.can_blur).then(|| {
                let (width, height) = (size.width.as_f32() as i32, size.height.as_f32() as i32);
                (
                    inset,
                    inset,
                    width - 2 * inset,
                    height - 2 * inset,
                    chrome.radius() as i32,
                )
            });
        if want == w.sent {
            return;
        }
        w.sent = want;
        let (Some(manager), Some(compositor)) = (&w.globals.manager, &w.globals.compositor) else {
            return;
        };
        let qh = w.queue.handle();
        let effect = w
            .effect
            .get_or_insert_with(|| manager.get_background_effect(&w.surface, &qh, ()));
        match want {
            Some((x, y, width, height, r)) => {
                let region = compositor.create_region(&qh, ());
                region.add(x, y, width, height);
                // Cut the rounded corners out, one pixel row at a time.
                for row in 0..r {
                    let dy = r as f32 - row as f32 - 0.5;
                    let dx = r - ((r * r) as f32 - dy * dy).sqrt() as i32;
                    for (cx, cy) in [(x, y + row), (x + width - dx, y + row)] {
                        region.subtract(cx, cy, dx, 1);
                        region.subtract(cx, 2 * y + height - 1 - cy, dx, 1);
                    }
                }
                effect.set_blur_region(Some(&region));
                region.destroy();
            }
            None => effect.set_blur_region(None),
        }
        if let Err(e) = w.queue.flush() {
            log::warn!("cannot set the blur region: {e}");
        }
    }
}

impl Wayland {
    fn connect(window: &Window) -> anyhow::Result<Self> {
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) = (
            window.display_handle()?.as_raw(),
            HasWindowHandle::window_handle(window)?.as_raw(),
        ) else {
            anyhow::bail!("not a Wayland window");
        };
        // Safety: gpui owns the display and the surface and keeps both alive as long as the window.
        let conn = Connection::from_backend(unsafe {
            Backend::from_foreign_display(display.display.as_ptr().cast())
        });
        let id = unsafe {
            ObjectId::from_ptr(
                wl_surface::WlSurface::interface(),
                surface.surface.as_ptr().cast(),
            )
        }?;
        let surface = wl_surface::WlSurface::from_id(&conn, id)?;
        let mut queue = conn.new_event_queue();
        conn.display().get_registry(&queue.handle(), ());
        let mut globals = Globals::default();
        // First the globals, then the manager's capabilities.
        queue.roundtrip(&mut globals)?;
        queue.roundtrip(&mut globals)?;
        if globals.manager.is_none() {
            anyhow::bail!("the compositor has no ext_background_effect_manager_v1");
        }
        Ok(Self {
            queue,
            globals,
            surface,
            effect: None,
            sent: None,
        })
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for Globals {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name, interface, ..
        } = event
        else {
            return;
        };
        if interface == wl_compositor::WlCompositor::interface().name {
            state.compositor = Some(registry.bind(name, 1, qh, ()));
        } else if interface == manager::ExtBackgroundEffectManagerV1::interface().name {
            state.manager = Some(registry.bind(name, 1, qh, ()));
        }
    }
}

impl Dispatch<manager::ExtBackgroundEffectManagerV1, ()> for Globals {
    fn event(
        state: &mut Self,
        _: &manager::ExtBackgroundEffectManagerV1,
        event: manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let manager::Event::Capabilities { flags } = event {
            state.can_blur = flags
                .into_result()
                .is_ok_and(|f| f.contains(manager::Capability::Blur));
        }
    }
}

delegate_noop!(Globals: ignore wl_compositor::WlCompositor);
delegate_noop!(Globals: ignore wl_region::WlRegion);
delegate_noop!(Globals: ignore effect::ExtBackgroundEffectSurfaceV1);
