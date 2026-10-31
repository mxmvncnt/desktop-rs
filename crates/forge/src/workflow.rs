//! Stages for platforms whose jobs have none (GitHub, Gitea, Forgejo): one column per depth in the `needs` graph.

use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml_ng::Value;

use crate::model::*;

#[derive(Deserialize)]
struct Workflow {
    jobs: HashMap<String, WorkflowJob>,
}

#[derive(Deserialize)]
struct WorkflowJob {
    name: Option<String>,
    needs: Option<Needs>,
    #[serde(rename = "continue-on-error")]
    continue_on_error: Option<Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Needs {
    One(String),
    Many(Vec<String>),
}

/// Group `jobs` into columns by the depth of their `needs` in the workflow file `yaml`. Jobs are matched by the
/// `name:` (or key) of the workflow job, or by that name followed by a matrix (`build (ubuntu, 1.80)`) or a
/// reusable workflow (`build / inner`). Also fills `needs` and `allow_failure` (`continue-on-error: true`).
/// Without a parsable file, everything goes in one stage.
pub fn stages_from_workflow(yaml: &str, mut jobs: Vec<Job>) -> Vec<Stage> {
    let Ok(workflow) = serde_yaml_ng::from_str::<Workflow>(yaml) else {
        return vec![Stage { name: "Jobs".into(), status: stage_status(&jobs), jobs }];
    };
    let display = |key: &str| workflow.jobs.get(key).map_or_else(|| key.to_string(), |def| display_of(key, def));
    for job in &mut jobs {
        let found = workflow.jobs.iter().find(|(key, def)| is_instance(&job.name, &display_of(key, def)));
        if let Some((_, def)) = found {
            let needs = match &def.needs {
                Some(Needs::One(n)) => std::slice::from_ref(n),
                Some(Needs::Many(n)) => n.as_slice(),
                None => &[],
            };
            job.needs = needs.iter().map(|n| display(n)).collect();
            job.allow_failure |= def.continue_on_error == Some(Value::Bool(true));
        }
    }
    group_by_needs(jobs)
}

fn display_of(key: &str, def: &WorkflowJob) -> String {
    def.name.clone().unwrap_or_else(|| key.to_string())
}

/// `job` is the workflow job called `base`, possibly expanded by a matrix or a reusable workflow.
fn is_instance(job: &str, base: &str) -> bool {
    job.strip_prefix(base).is_some_and(|rest| rest.is_empty() || rest.starts_with(" (") || rest.starts_with(" / "))
}

/// Columns "Stage 1", "Stage 2", ...: a job sits one column right of the deepest job it needs.
pub(crate) fn group_by_needs(jobs: Vec<Job>) -> Vec<Stage> {
    let deps: Vec<Vec<usize>> = (0..jobs.len())
        .map(|i| {
            let needed = |(j, other): &(usize, &Job)| *j != i && jobs[i].needs.iter().any(|need| is_instance(&other.name, need));
            jobs.iter().enumerate().filter(needed).map(|(j, _)| j).collect()
        })
        .collect();
    // Relax n times: enough for any acyclic graph, and bounded if the file has a cycle.
    let mut depth = vec![0; jobs.len()];
    for _ in 0..jobs.len() {
        for i in 0..jobs.len() {
            depth[i] = deps[i].iter().map(|&d| depth[d] + 1).max().unwrap_or(0);
        }
    }
    let columns = depth.iter().max().map_or(0, |d| d + 1);
    let mut stages: Vec<Stage> = (1..=columns).map(|n| Stage { name: format!("Stage {n}"), status: Status::Created, jobs: Vec::new() }).collect();
    for (mut job, d) in jobs.into_iter().zip(depth) {
        job.stage = Some(stages[d].name.clone());
        stages[d].jobs.push(job);
    }
    for stage in &mut stages {
        stage.status = stage_status(&stage.jobs);
    }
    stages
}

#[cfg(test)]
#[path = "tests/workflow.test.rs"]
mod tests;
