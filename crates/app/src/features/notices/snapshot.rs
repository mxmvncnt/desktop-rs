//! Snapshot views: the error banner.

use crate::snapshot::View;

pub fn views() -> &'static [View] {
    &[View {
        name: "notice-error",
        steps: &[|ws, _, _, cx| {
            if let Some(r) = &ws.repo {
                r.update(cx, |r, _| {
                    r.error = Some(
                        "fatal: path 'src/combo_field.rs' exists on disk, but not in 'HEAD'".into(),
                    )
                });
            }
            cx.notify();
        }],
    }]
}
