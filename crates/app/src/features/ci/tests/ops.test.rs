use super::*;

#[test]
fn workflow_path_with_a_directory_is_used_as_is() {
    assert_eq!(workflow_candidates(".github/workflows/ci.yml"), [".github/workflows/ci.yml"]);
}

#[test]
fn bare_workflow_name_is_looked_up_in_every_workflow_folder() {
    assert_eq!(
        workflow_candidates("ci.yaml"),
        [".forgejo/workflows/ci.yaml", ".gitea/workflows/ci.yaml", ".github/workflows/ci.yaml"]
    );
}

fn log_with(sections: Vec<(usize, usize, bool)>, lines: usize) -> Log {
    Log {
        lines: vec![gd_forge::Line::default(); lines],
        sections: sections
            .into_iter()
            .map(|(header, end, collapsed)| gd_forge::Section { name: String::new(), header, end, duration: None, collapsed, depth: 0 })
            .collect(),
    }
}

#[test]
fn collapsed_sections_hide_their_body() {
    let log = log_with(vec![(1, 4, true)], 6);
    assert_eq!(visible_lines(&log, &HashSet::new()), [0, 1, 4, 5]);
}

#[test]
fn toggling_flips_the_default() {
    let log = log_with(vec![(1, 4, true), (4, 6, false)], 6);
    assert_eq!(visible_lines(&log, &HashSet::from([0, 1])), [0, 1, 2, 3, 4]);
}

#[test]
fn a_collapsed_parent_hides_nested_sections() {
    let log = log_with(vec![(0, 5, true), (1, 3, false)], 6);
    assert_eq!(visible_lines(&log, &HashSet::new()), [0, 5]);
}
