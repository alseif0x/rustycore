use super::*;

#[test]
fn wrapper_without_cast_authority_fails_closed_and_preserves_snapshot() {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100, 200],
        effects: vec![learn_effect(1, 200, 0, 100)],
        ..Default::default()
    });
    let input = snapshot();
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::TrainerWrapperCast(200),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::CastAuthority {
                spell_id: 200,
                reasons: vec![SpellAcquisitionCastIndeterminateReasonLikeCpp::IncompleteAuthority,],
            },
        )
    );
}

#[test]
fn missing_spell_and_unhydrated_skill_metadata_fail_closed() {
    const MISSING_SPELL: u32 = 999;
    let empty_metadata = MetadataFixture::new(FixtureInput::default());
    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot(),
            empty_metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(MISSING_SPELL),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::MissingSpellCoverage {
                spell_id: MISSING_SPELL,
                table: SpellAcquisitionTableLikeCpp::SpellEffect,
            },
        )
    );

    const ROOT: u32 = 500;
    const INCOMPLETE_SKILL: u32 = 164;
    let incomplete_metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ROOT],
        learn_skills: vec![(
            ROOT,
            SpellLearnSkillNodeLikeCpp {
                skill: INCOMPLETE_SKILL as u16,
                step: 1,
                value: 1,
                maxvalue: 75,
            },
        )],
        incomplete_skill_line_ids: vec![INCOMPLETE_SKILL],
        ..Default::default()
    });
    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot(),
            incomplete_metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ROOT),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::IncompleteSkillLine {
                skill_id: INCOMPLETE_SKILL,
            },
        )
    );
}

#[test]
fn complete_empty_mount_authority_allows_an_ordinary_spell() {
    const ORDINARY_SPELL: u32 = 500;
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ORDINARY_SPELL],
        ..Default::default()
    });

    let plan = deterministic(project_spell_acquisition_like_cpp(
        &snapshot(),
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(ORDINARY_SPELL),
    ));

    assert!(
        plan.resulting_snapshot
            .spells
            .iter()
            .any(|spell| spell.spell_id == ORDINARY_SPELL)
    );
}

#[test]
fn missing_mount_authority_fails_closed_for_an_ordinary_spell() {
    const ORDINARY_SPELL: u32 = 500;
    let mut metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![ORDINARY_SPELL],
        ..Default::default()
    });
    metadata.mounts_complete = false;

    assert_eq!(
        project_spell_acquisition_like_cpp(
            &snapshot(),
            metadata.metadata(),
            SpellAcquisitionRootLikeCpp::DirectLearn(ORDINARY_SPELL),
        ),
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::IncompleteMountAuthority {
                spell_id: ORDINARY_SPELL,
            },
        )
    );
}

#[test]
fn mount_source_spell_fails_closed_before_exposing_its_faction_learn_edge() {
    const MOUNT_SPELL: u32 = 500;
    const OTHER_FACTION_SPELL: u32 = 600;
    let mut metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![MOUNT_SPELL, OTHER_FACTION_SPELL],
        dependencies: vec![dependency(1, MOUNT_SPELL, OTHER_FACTION_SPELL)],
        ..Default::default()
    });
    metadata.mounts = MountStore::from_entries([MountEntry {
        id: 1,
        mount_type_id: 1,
        flags: 0,
        source_type_enum: 0,
        source_spell_id: MOUNT_SPELL as i32,
        player_condition_id: 0,
        mount_fly_ride_height: 0.0,
        ui_model_scene_id: 0,
    }]);
    let input = snapshot();
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(MOUNT_SPELL),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::UnsupportedMountAcquisition {
                spell_id: MOUNT_SPELL,
            },
        )
    );
}

#[test]
fn negative_trait_override_spell_id_fails_closed_without_mutating_input() {
    const SPELL: u32 = 500;
    const TRAIT_DEFINITION: u32 = 7;
    let mut metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![SPELL],
        ..Default::default()
    });
    metadata.trait_definitions =
        TraitDefinitionStore::from_entries([wow_data::trait_tree::TraitDefinitionEntry {
            id: TRAIT_DEFINITION,
            override_name: String::new(),
            override_subtext: String::new(),
            override_description: String::new(),
            spell_id: SPELL as i32,
            override_icon: 0,
            overrides_spell_id: -1,
            visible_spell_id: 0,
        }]);
    let mut input = snapshot();
    let mut existing = spell_row(SPELL, false, false);
    existing.trait_definition_id = Some(TRAIT_DEFINITION as i32);
    input.spells.push(existing);
    let before = input.clone();

    let outcome = project_spell_acquisition_like_cpp(
        &input,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(SPELL),
    );

    assert_eq!(input, before);
    assert_eq!(
        outcome,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(
            SpellAcquisitionIndeterminateLikeCpp::InvalidEffectiveValue {
                record_id: TRAIT_DEFINITION,
                field: "TraitDefinition.OverridesSpellID",
                raw: -1,
            }
        )
    );
}
