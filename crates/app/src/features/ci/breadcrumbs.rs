//! The trail of the pipeline and job pages: "Pipelines", the pipeline number and, on a job, its name.

use gpui::Entity;

use super::state::CiStore;
use crate::components::{Breadcrumbs, Crumb};

/// `job`: name of the open job; `None` on the pipeline page.
pub fn breadcrumbs(store: &Entity<CiStore>, pipeline: u64, job: Option<String>) -> Breadcrumbs {
    let (back, to_list, to_pipeline) = (store.clone(), store.clone(), store.clone());
    let number = format!("#{pipeline}");
    let mut crumbs = vec![Crumb::link("Pipelines", move |_, cx| {
        to_list.update(cx, |s, cx| s.show_list(cx))
    })];
    match job {
        Some(name) => crumbs.extend([
            Crumb::link(number, move |_, cx| {
                to_pipeline.update(cx, |s, cx| s.back(cx))
            }),
            Crumb::here(name),
        ]),
        None => crumbs.push(Crumb::here(number)),
    }
    Breadcrumbs {
        id: "ci-crumb",
        on_back: std::rc::Rc::new(move |_, cx| back.update(cx, |s, cx| s.back(cx))),
        crumbs,
    }
}
