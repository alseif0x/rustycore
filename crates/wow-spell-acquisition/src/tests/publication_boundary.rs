use super::*;

#[test]
fn caller_action_edits_do_not_replace_planner_publication_authority() {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100],
        ..Default::default()
    });
    let mut plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(100),
    ));
    let authority = plan.publication_requirements_like_cpp().to_vec();
    assert!(!authority.is_empty());
    plan.post_commit_actions.clear();
    assert_eq!(
        plan.publication_requirements_like_cpp(),
        authority.as_slice()
    );
}
