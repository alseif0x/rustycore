use super::*;
use std::sync::Arc;

#[test]
fn input_admission_requires_class_lookup_ranks_and_same_shared_birth_authority() {
    let fixture = Fixture::new(vec![line(10, 6, 0, 0)], vec![]);
    for class in [0, 16, 255] {
        assert!(matches!(
            SkillSetSources::new(&fixture.world, &fixture.spells, 1, class),
            Err(SkillSetInputError::World(SourceError::InvalidClass))
        ));
    }
    let definitions =
        crate::forever::spells::skill_set_test_definitions(Arc::clone(&fixture.birth));
    assert!(matches!(
        SkillSetSources::new(&fixture.world, &definitions, 1, 1),
        Err(SkillSetInputError::MissingRanks)
    ));
    let other = Fixture::new(vec![line(10, 6, 0, 0)], vec![]);
    assert!(matches!(
        SkillSetSources::new(&fixture.world, &other.spells, 1, 1),
        Err(SkillSetInputError::MismatchedBirthCatalog)
    ));
}
