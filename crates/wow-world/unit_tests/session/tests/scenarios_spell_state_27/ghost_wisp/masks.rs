use super::*;

fn aura_row(
    spell_id: i32,
    caster_guid: ObjectGuid,
    item_guid: ObjectGuid,
    effect_mask: u32,
    recalculate_mask: u32,
) -> wow_world_spell::CharacterAuraRowLikeCpp {
    wow_world_spell::CharacterAuraRowLikeCpp {
        caster_guid,
        item_guid,
        spell_id: spell_id as u32,
        effect_mask,
        recalculate_mask,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: -1,
        remain_time_ms: -1,
        remain_charges: 3,
        cast_item_id: 0,
        cast_item_level: -1,
    }
}

fn effect_rows(
    spell_id: i32,
    caster_guid: ObjectGuid,
    item_guid: ObjectGuid,
    effect_mask: u32,
    values: &[(u8, i32, i32)],
) -> Vec<wow_world_spell::CharacterAuraEffectRowLikeCpp> {
    values
        .iter()
        .map(
            |(effect_index, amount, base_amount)| wow_world_spell::CharacterAuraEffectRowLikeCpp {
                caster_guid,
                item_guid,
                spell_id: spell_id as u32,
                effect_mask,
                effect_index: *effect_index,
                amount: *amount,
                base_amount: *base_amount,
            },
        )
        .collect()
}

fn load(
    fixture: &mut GhostFixture,
    aura_rows: impl IntoIterator<Item = wow_world_spell::CharacterAuraRowLikeCpp>,
    effect_rows: impl IntoIterator<Item = wow_world_spell::CharacterAuraEffectRowLikeCpp>,
) -> usize {
    fixture
        .session
        .load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0)
}

fn set_authority(fixture: &mut GhostFixture, complete: bool) {
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            player.set_player_aura_authority_complete_like_cpp(complete);
        })
        .expect("canonical Player");
}

fn applications(fixture: &GhostFixture) -> Vec<wow_entities::AuraApplicationLikeCpp> {
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras
        .runtime_applications_like_cpp()
        .values()
        .cloned()
        .collect()
}

fn aura_authority(fixture: &GhostFixture) -> bool {
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras
        .persisted_player_aura_authority_complete_like_cpp()
}

#[tokio::test]
async fn loaded_ghost_speed_subset_obeys_spell_debuff_and_does_not_set_ghost_flag() {
    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    let row = aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b110, 0);
    let effects = effect_rows(
        GHOST,
        fixture.player,
        ObjectGuid::EMPTY,
        0b110,
        &[(1, 50, 49), (2, 50, 49)],
    );

    assert_eq!(load(&mut fixture, [row], effects), 1);
    let loaded = applications(&fixture);
    let [ghost] = loaded.as_slice() else {
        panic!("one selected partial Ghost application");
    };
    assert_eq!(ghost.effect_mask, 0b110);
    assert_eq!(
        ghost.aura_flags,
        0x0011,
        "SpellInfo AURA_IS_DEBUFF precedes the speed aura effect polarity",
    );
    assert_eq!(
        ghost.aura_flags & 0x0002,
        0,
        "a debuff Aura is not cancelable",
    );
    assert!(
        !is_ghost_flag_set(&fixture),
        "effect 0 / aura 95 was inactive"
    );

    fixture
        .session
        .remove_aura(ghost.slot)
        .expect("remove speed subset");
    assert!(!is_ghost_flag_set(&fixture));
    assert!(aura_authority(&fixture));
}

#[tokio::test]
async fn active_ghost_flag_clears_only_after_the_last_ghost_effect_application() {
    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    let row = aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b001, 0);
    let effects = effect_rows(
        GHOST,
        fixture.player,
        ObjectGuid::EMPTY,
        0b001,
        &[(0, -20, -21)],
    );
    assert_eq!(load(&mut fixture, [row], effects), 1);
    assert!(is_ghost_flag_set(&fixture));
    let primary = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == GHOST)
        .expect("active Ghost effect");
    assert_eq!(
        primary.aura_flags, 0x0011,
        "self negative aura carries NOCASTER"
    );

    let other_caster = ObjectGuid::create_player(1, 96_120);
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let mut second = primary.clone();
            second.slot = 7;
            second.caster_guid = other_caster;
            let snapshot = auras
                .threat_snapshot_like_cpp(primary.slot)
                .expect("primary Ghost effect snapshot")
                .clone();
            player.install_player_threat_aura_like_cpp(7, snapshot.clone(), second);
            player.apply_player_threat_aura_like_cpp(GHOST as u32, other_caster, 7, snapshot);
        })
        .expect("canonical Player");

    fixture
        .session
        .remove_aura(primary.slot)
        .expect("remove first source");
    assert!(
        is_ghost_flag_set(&fixture),
        "another active aura 95 remains"
    );
    fixture.session.remove_aura(7).expect("remove final source");
    assert!(!is_ghost_flag_set(&fixture));

    let mut foreign = super::fixture();
    set_authority(&mut foreign, true);
    let caster = ObjectGuid::create_player(1, 96_121);
    let row = aura_row(GHOST, caster, ObjectGuid::EMPTY, 0b001, 0);
    let effects = effect_rows(GHOST, caster, ObjectGuid::EMPTY, 0b001, &[(0, -20, -21)]);
    assert_eq!(load(&mut foreign, [row], effects), 1);
    let application = applications(&foreign).pop().expect("foreign Ghost");
    assert_eq!(
        application.aura_flags, 0x0010,
        "foreign source has no NOCASTER flag"
    );
    assert!(is_ghost_flag_set(&foreign));
    foreign
        .session
        .remove_aura(application.slot)
        .expect("remove foreign source");
    assert!(!is_ghost_flag_set(&foreign));
}

#[tokio::test]
async fn loaded_wisp_speed_subset_preserves_another_transform() {
    let mut fixture = fixture();
    let mut negative = aura_spell(
        NEGATIVE_TRANSFORM,
        [(wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, 0, 12_862)],
    );
    negative.effects[0].implicit_target_1 = 2;
    install_spell_catalog(&mut fixture.session, [negative]);
    install_wisp_templates(
        &mut fixture.session,
        &[(WISP_TEMPLATE, 1_825), (12_862, 1_826)],
    );
    fixture
        .session
        .apply_aura(NEGATIVE_TRANSFORM, fixture.player, 0, 0x0010)
        .expect("apply existing negative transform");
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_display_id(1_826, true))
        .expect("canonical Player");
    assert_eq!(player_display(&fixture), 1_826);
    let other_transform_slot = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == NEGATIVE_TRANSFORM)
        .expect("existing transform");

    set_authority(&mut fixture, true);
    let row = aura_row(WISP, fixture.player, ObjectGuid::EMPTY, 0b011, 0);
    let effects = effect_rows(
        WISP,
        fixture.player,
        ObjectGuid::EMPTY,
        0b011,
        &[(0, 75, 74), (1, 75, 74)],
    );
    assert_eq!(load(&mut fixture, [row], effects), 1);
    assert_eq!(player_display(&fixture), 1_826);
    let wisp = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == WISP)
        .expect("Wisp speed subset");
    assert_eq!(wisp.effect_mask, 0b011);
    assert!(
        !fixture
            .canonical
            .lock()
            .expect("canonical manager")
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .applied_aura_types
            .get(&wow_data::spell::aura_types::SPELL_AURA_TRANSFORM)
            .is_some_and(|effects| effects.iter().any(|effect| effect.slot == wisp.slot))
    );

    fixture
        .session
        .remove_aura(wisp.slot)
        .expect("remove Wisp speed subset");
    assert_eq!(player_display(&fixture), 1_826);
    assert!(
        applications(&fixture)
            .iter()
            .any(|aura| aura.slot == other_transform_slot.slot)
    );
}

#[tokio::test]
async fn loaded_wisp_transform_without_recalculation_uses_stored_amount() {
    let mut fixture = fixture();
    fixture
        .session
        .set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    set_authority(&mut fixture, true);
    let row = aura_row(WISP, fixture.player, ObjectGuid::EMPTY, 0b100, 0);
    let effects = effect_rows(
        WISP,
        fixture.player,
        ObjectGuid::EMPTY,
        0b100,
        &[(2, 17, -1)],
    );
    assert_eq!(load(&mut fixture, [row], effects), 1);
    let wisp = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == WISP)
        .expect("Wisp transform subset");
    assert_eq!(wisp.effect_mask, 0b100);
    assert_eq!(wisp.represented_effect_amounts[0].amount, 17);
    assert_eq!(player_display(&fixture), 1_825);
    let saved_wisp = fixture
        .session
        .prepare_player_save_like_cpp(1_700_000_000)
        .expect("canonical full-save owner")
        .request
        .ghost_wisp
        .expect("complete selected authority")
        .into_iter()
        .find(|family| {
            family.spell_id == WISP as u32
                && family.caster_guid_binary == fixture.player.to_raw_bytes()
                && family.item_guid_binary == ObjectGuid::EMPTY.to_raw_bytes()
        })
        .expect("self Wisp family");
    assert_eq!(saved_wisp.auras[0].recalculate_mask, 0);
    assert_eq!(saved_wisp.auras[0].effects[0].amount, 17);

    fixture
        .session
        .remove_aura(wisp.slot)
        .expect("remove Wisp transform");
    assert_eq!(player_display(&fixture), 1_000);
}

#[tokio::test]
async fn loaded_wisp_transform_with_recalculation_uses_base_and_clears_runtime_bit() {
    let mut fixture = fixture();
    fixture
        .session
        .set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    set_authority(&mut fixture, true);
    let row = aura_row(WISP, fixture.player, ObjectGuid::EMPTY, 0b100, 0b100);
    let effects = effect_rows(
        WISP,
        fixture.player,
        ObjectGuid::EMPTY,
        0b100,
        &[(2, 17, -1)],
    );

    assert_eq!(load(&mut fixture, [row], effects), 1);
    let wisp = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == WISP)
        .expect("Wisp transform subset");
    assert_eq!(wisp.effect_mask, 0b100);
    assert_eq!(wisp.represented_effect_amounts[0].amount, 0);
    let metadata = fixture
        .session
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_persistence_metadata(wisp.slot)
                .cloned()
                .expect("selected persistence metadata")
        })
        .expect("canonical Player");
    assert_eq!(metadata.recalculate_mask, 0);
    assert_eq!(metadata.base_amounts, [(2, -1)]);
    let saved_wisp = fixture
        .session
        .prepare_player_save_like_cpp(1_700_000_000)
        .expect("canonical full-save owner")
        .request
        .ghost_wisp
        .expect("complete selected authority")
        .into_iter()
        .find(|family| {
            family.spell_id == WISP as u32
                && family.caster_guid_binary == fixture.player.to_raw_bytes()
                && family.item_guid_binary == ObjectGuid::EMPTY.to_raw_bytes()
        })
        .expect("self Wisp family");
    assert_eq!(saved_wisp.auras[0].recalculate_mask, 0);
    assert_eq!(saved_wisp.auras[0].effects[0].amount, 0);
}

#[tokio::test]
async fn selected_loader_normalizes_raw_mask_and_recaptures_the_normalized_key() {
    let mut fixture = fixture();
    fixture
        .session
        .set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    set_authority(&mut fixture, true);
    let raw_mask = 0b1001;
    let row = aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, raw_mask, 0b1000);
    let effects = effect_rows(
        GHOST,
        fixture.player,
        ObjectGuid::EMPTY,
        raw_mask,
        &[(0, -13, -21), (3, 999, 888)],
    );
    assert_eq!(load(&mut fixture, [row], effects), 1);
    let ghost = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == GHOST)
        .expect("normalized Ghost aura");
    assert_eq!(ghost.effect_mask, 0b001);
    assert_eq!(ghost.represented_effect_amounts.len(), 1);
    assert_eq!(ghost.represented_effect_amounts[0].amount, -13);
    assert!(aura_authority(&fixture));
    let (loaded_metadata, loaded_effects) = fixture
        .session
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            (
                auras
                    .aura_persistence_metadata(ghost.slot)
                    .cloned()
                    .expect("selected persistence metadata"),
                auras
                    .threat_snapshot_like_cpp(ghost.slot)
                    .expect("selected threat snapshot")
                    .effects()
                    .to_vec(),
            )
        })
        .expect("canonical Player");
    assert_eq!(loaded_metadata.recalculate_mask, 0);
    assert_eq!(loaded_metadata.base_amounts, [(0, -21)]);
    assert_eq!(loaded_effects.len(), 1);
    assert_eq!(loaded_effects[0].0, 0b001);

    let families = fixture
        .session
        .prepare_player_save_like_cpp(1_700_000_000)
        .expect("canonical full-save owner")
        .request
        .ghost_wisp
        .expect("complete selected authority");
    let family = families
        .iter()
        .find(|family| {
            family.spell_id == GHOST as u32
                && family.caster_guid_binary == fixture.player.to_raw_bytes()
                && family.item_guid_binary == ObjectGuid::EMPTY.to_raw_bytes()
        })
        .expect("normalized empty-item Ghost family");
    assert_eq!(family.auras.len(), 1);
    assert_eq!(family.auras[0].recalculate_mask, 0);
    assert_eq!(
        family.auras[0]
            .effects
            .iter()
            .map(|effect| (effect.index, effect.amount, effect.base_amount))
            .collect::<Vec<_>>(),
        [(0, -13, -21)]
    );
}

#[tokio::test]
async fn known_zero_and_unknown_only_masks_consume_cast_ids_then_skip() {
    let mut expected = fixture();
    let expected_ids = (0..3)
        .map(|_| {
            wow_world_spell::next_represented_spell_cast_guid_like_cpp(&expected.session, GHOST)
                .expect("next Cast GUID")
        })
        .collect::<Vec<_>>();

    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    let rows = [
        aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0, 0),
        aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b1000, 0b1000),
        aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b001, 0),
    ];
    let effects = effect_rows(
        GHOST,
        fixture.player,
        ObjectGuid::EMPTY,
        0b001,
        &[(0, -20, -21)],
    );
    assert_eq!(load(&mut fixture, rows, effects), 1);
    let ghost = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == GHOST)
        .expect("valid mask after skipped rows");
    let cast_id = fixture
        .session
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(ghost.slot)
                .cast_id
        })
        .expect("canonical Player");
    assert_eq!(cast_id, expected_ids[2]);
    assert!(aura_authority(&fixture));
}

#[tokio::test]
async fn load_replaces_same_spell_even_for_same_caster_and_mask_before_item_skip() {
    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    let item_guid = ObjectGuid::create_item(1, 96_122);
    let rows = [
        aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b001, 0),
        aura_row(GHOST, fixture.player, item_guid, 0b001, 0),
    ];
    let effects = [
        effect_rows(
            GHOST,
            fixture.player,
            ObjectGuid::EMPTY,
            0b001,
            &[(0, -20, -21)],
        ),
        effect_rows(GHOST, fixture.player, item_guid, 0b001, &[(0, -20, -21)]),
    ]
    .concat();

    assert_eq!(load(&mut fixture, rows, effects), 1);
    assert!(applications(&fixture).is_empty());
    assert!(!is_ghost_flag_set(&fixture));
    assert!(aura_authority(&fixture));
}

#[tokio::test]
async fn foreign_limit_n_aura_is_skipped_after_replacing_existing_same_spell() {
    let mut fixture = fixture();
    let ghost = aura_spell(
        GHOST,
        [
            (wow_data::spell::aura_types::SPELL_AURA_GHOST, -21, 0),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                49,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                49,
                0,
            ),
        ],
    );
    let wisp = aura_spell(
        WISP,
        [
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                74,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                74,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                -1,
                WISP_TEMPLATE as i32,
            ),
        ],
    );
    let mut store = crate::selected_spell_fixtures::canonical_spell_store([
        (GHOST as u32, 0),
        (WISP as u32, 0),
    ]);
    let mut ghost_attributes = selected_aura_attributes(GHOST);
    ghost_attributes[5] = 0x20;
    store.insert_spell_misc_attributes_like_cpp(GHOST, ghost_attributes);
    store.insert_spell_misc_attributes_like_cpp(WISP, selected_aura_attributes(WISP));
    store.insert(GHOST, ghost);
    store.insert(WISP, wisp);
    fixture.session.set_spell_store(Arc::new(store));
    set_authority(&mut fixture, true);
    let foreign_caster = ObjectGuid::create_player(1, 96_123);
    let row = aura_row(GHOST, foreign_caster, ObjectGuid::EMPTY, 0b001, 0);
    let effects = effect_rows(
        GHOST,
        foreign_caster,
        ObjectGuid::EMPTY,
        0b001,
        &[(0, -20, -21)],
    );

    assert_eq!(load(&mut fixture, [row], effects), 0);
    assert!(applications(&fixture).is_empty());
    assert!(!is_ghost_flag_set(&fixture));
    assert!(aura_authority(&fixture));
}

#[tokio::test]
async fn loader_rejects_area_target_catalog_before_replacing_existing_ghost() {
    let mut fixture = fixture();
    fixture
        .session
        .set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    set_authority(&mut fixture, true);
    cast_selected(&mut fixture, GHOST, None).await;
    let before = applications(&fixture);
    let [existing] = before.as_slice() else {
        panic!("one existing Ghost application");
    };

    let mut area_ghost = aura_spell(
        GHOST,
        [
            (wow_data::spell::aura_types::SPELL_AURA_GHOST, -21, 0),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                49,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                49,
                0,
            ),
        ],
    );
    area_ghost.effects[0].implicit_target_1 = 7; // TARGET_UNIT_SRC_AREA_ENTRY.
    install_spell_catalog(&mut fixture.session, [area_ghost]);
    let row = aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b111, 0);
    let effects = effect_rows(
        GHOST,
        fixture.player,
        ObjectGuid::EMPTY,
        0b111,
        &[(0, -21, -21), (1, 50, 49), (2, 50, 49)],
    );

    assert_eq!(load(&mut fixture, [row], effects), 0);
    let after = applications(&fixture);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].slot, existing.slot);
    assert_eq!(after[0].caster_guid, existing.caster_guid);
    assert_eq!(after[0].effect_mask, existing.effect_mask);
    assert!(!aura_authority(&fixture));
}

#[tokio::test]
async fn fresh_self_application_replaces_an_existing_foreign_same_spell_aura() {
    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    let foreign_caster = ObjectGuid::create_player(1, 96_124);
    let row = aura_row(GHOST, foreign_caster, ObjectGuid::EMPTY, 0b111, 0b111);
    let effects = effect_rows(
        GHOST,
        foreign_caster,
        ObjectGuid::EMPTY,
        0b111,
        &[(0, -21, -21), (1, 50, 49), (2, 50, 49)],
    );
    assert_eq!(load(&mut fixture, [row], effects), 1);
    let previous = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == GHOST)
        .expect("loaded foreign Ghost");
    assert_eq!(previous.caster_guid, foreign_caster);

    cast_selected(&mut fixture, GHOST, None).await;
    let current = applications(&fixture);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].slot, previous.slot, "freed slot is reused");
    assert_eq!(current[0].caster_guid, fixture.player);
    assert!(is_ghost_flag_set(&fixture));
}

#[tokio::test]
async fn same_spell_different_mask_replaces_and_reuses_the_old_slot() {
    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    let rows = [
        aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b001, 0),
        aura_row(GHOST, fixture.player, ObjectGuid::EMPTY, 0b010, 0),
    ];
    let effects = [
        effect_rows(
            GHOST,
            fixture.player,
            ObjectGuid::EMPTY,
            0b001,
            &[(0, -20, -21)],
        ),
        effect_rows(
            GHOST,
            fixture.player,
            ObjectGuid::EMPTY,
            0b010,
            &[(1, 50, 49)],
        ),
    ]
    .concat();

    assert_eq!(load(&mut fixture, rows, effects), 2);
    let applications = applications(&fixture);
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].slot, 0);
    assert_eq!(applications[0].effect_mask, 0b010);
    assert!(!is_ghost_flag_set(&fixture));
}

#[tokio::test]
async fn missing_or_duplicate_active_effect_metadata_withholds_authority() {
    for effects in [
        Vec::new(),
        effect_rows(
            GHOST,
            ObjectGuid::create_player(1, 96_110),
            ObjectGuid::EMPTY,
            0b001,
            &[(0, -20, -21), (0, -19, -18)],
        ),
    ] {
        let mut fixture = fixture();
        set_authority(&mut fixture, true);
        let caster = fixture.player;
        let row = aura_row(GHOST, caster, ObjectGuid::EMPTY, 0b001, 0);
        assert_eq!(load(&mut fixture, [row], effects), 0);
        assert!(applications(&fixture).is_empty());
        assert!(!aura_authority(&fixture));
    }
}

#[tokio::test]
async fn direct_removal_rejects_unknown_bits_without_partial_effects() {
    let mut fixture = fixture();
    set_authority(&mut fixture, true);
    cast_selected(&mut fixture, GHOST, None).await;
    let _ = outgoing(&fixture);
    wow_world_spell::tick_auras(&mut fixture.session);
    let _ = outgoing(&fixture);
    let application = applications(&fixture)
        .into_iter()
        .find(|aura| aura.spell_id == GHOST)
        .expect("fresh Ghost");
    let run_speed = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let swim_speed = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    let removed_count = {
        let manager = fixture.canonical.lock().expect("canonical manager");
        let auras = &manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras;
        auras.removed_auras_count
    };
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(application.slot)
                .expect("Ghost application")
                .effect_mask = 0b1001;
        })
        .expect("canonical Player");

    assert!(fixture.session.remove_aura(application.slot).is_err());
    assert!(is_ghost_flag_set(&fixture));
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - run_speed).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - swim_speed).abs() < 0.0001);
    let manager = fixture.canonical.lock().expect("canonical manager");
    let auras = &manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras;
    assert_eq!(auras.removed_auras_count, removed_count);
    assert_eq!(
        auras
            .runtime_application_like_cpp(application.slot)
            .expect("unchanged Ghost application")
            .effect_mask,
        0b1001
    );
}
