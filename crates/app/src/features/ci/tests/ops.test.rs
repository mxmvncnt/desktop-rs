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
