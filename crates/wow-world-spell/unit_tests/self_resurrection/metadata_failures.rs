use super::fixtures::*;
use crate::self_resurrection::{SelfResurrectionError, initialize_self_resurrection_spells};
use wow_data::{DifficultyEntry, DifficultyStore, ServersideSpellStoreLikeCpp, SpellStore};
use wow_entities::Player;

fn assert_unchanged_after_preflight_error(
    player: &Player,
    expected: &[i32],
    update_mask: &Option<Vec<u32>>,
) {
    assert_eq!(player.self_res_spells(), expected);
    assert_eq!(
        &player.active_data().self_res_spells_update_mask,
        update_mask
    );
}

#[test]
fn unknown_empty_aura_owner_fails_before_list_clear_or_rng() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_201);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_persisted_player_aura_authority_complete_like_cpp(false);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::AuraStateUnavailable)
    );
    assert_unchanged_after_preflight_error(&player, &[99_201], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn active_unknown_spell_key_fails_before_list_clear_or_rng() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_202);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    add_aura(&mut player, 0xF123_4567u32 as i32, 0, 7, 1);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::SpellInfoKeyUnavailable {
            spell_id: 0xF123_4567,
            requested_difficulty: 0,
        })
    );
    assert_unchanged_after_preflight_error(&player, &[99_202], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn missing_active_effect_slot_fails_before_list_clear_or_rng() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_203);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_203,
        0,
        0,
        0,
        [0; 4],
        vec![server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4])],
    );
    add_aura(&mut player, 71_203, 0, 7, 0b10);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &serverside,
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::ActiveEffectUnavailable {
            spell_id: 71_203,
            effect_index: 1,
        })
    );
    assert_unchanged_after_preflight_error(&player, &[99_203], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn active_bit_for_non_aura_effect_is_malformed_but_inactive_bits_are_ignored() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_204);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_204,
        0,
        0,
        0,
        [0; 4],
        vec![server_effect(0, 0, 0, [0; 4])],
    );
    add_aura(&mut player, 71_204, 0, 7, 1);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &serverside,
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::ActiveEffectNotAura {
            spell_id: 71_204,
            effect_index: 0,
        })
    );
    assert_unchanged_after_preflight_error(&player, &[99_204], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn regular_aura_effects_resolve_from_the_actual_nonzero_info_key() {
    let mut player = player_with_rows([], true);
    let spells = regular_spell_store(
        [(TWISTING_NETHER, 3)],
        [regular_effect(
            1,
            TWISTING_NETHER,
            3,
            0,
            APPLY_AURA,
            DUMMY_AURA as i16,
            [0; 4],
        )],
        [],
    );
    assert!(spells.get(TWISTING_NETHER as i32).is_none());
    add_aura(&mut player, TWISTING_NETHER as i32, 3, 4, 1);

    let mut rolls = 0;
    initialize_self_resurrection_spells(
        &mut player,
        0,
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        Some(&class_options([])),
        None,
        |_| {
            rolls += 1;
            true
        },
    )
    .unwrap();

    assert_eq!(rolls, 1);
    assert_eq!(player.self_res_spells(), [23_700]);
}

#[test]
fn regular_dummy_without_class_options_fails_before_list_clear_or_rng() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_211);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let spells = regular_spell_store(
        [(TWISTING_NETHER, 3)],
        [regular_effect(
            1,
            TWISTING_NETHER,
            3,
            0,
            APPLY_AURA,
            DUMMY_AURA as i16,
            [0; 4],
        )],
        [],
    );
    add_aura(&mut player, TWISTING_NETHER as i32, 3, 4, 1);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &spells,
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::ClassOptionsUnavailable {
            spell_id: TWISTING_NETHER,
        })
    );
    assert_unchanged_after_preflight_error(&player, &[99_211], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn joint_resolution_prefers_exact_server_key_over_regular_fallback_key() {
    let mut player = player_with_rows([], true);
    let spells = wow_world_test_support::spell_catalog::canonical_spell_store([(71_205, 0)]);
    let difficulties = DifficultyStore::from_entries([
        DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 4,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_205,
        4,
        0,
        5,
        [0, 0x0100_0000, 0, 0],
        vec![server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4])],
    );
    add_aura(&mut player, 71_205, 4, 7, 1);
    let mut rolls = 0;

    initialize_self_resurrection_spells(
        &mut player,
        0,
        &spells,
        &serverside,
        None,
        Some(&difficulties),
        |_| {
            rolls += 1;
            true
        },
    )
    .unwrap();

    assert_eq!(rolls, 0);
    assert_eq!(player.self_res_spells(), [3026]);
}

#[test]
fn same_exact_regular_and_server_key_is_rejected_before_list_clear_or_rng() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_206);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let spells = wow_world_test_support::spell_catalog::canonical_spell_store([(71_206, 0)]);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_206,
        0,
        0,
        0,
        [0; 4],
        vec![server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4])],
    );
    add_aura(&mut player, 71_206, 0, 7, 1);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &spells,
            &serverside,
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::SpellInfoSourceCollision {
            spell_id: 71_206,
            difficulty: 0,
        })
    );
    assert_unchanged_after_preflight_error(&player, &[99_206], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn regular_key_fallback_without_required_difficulty_graph_fails_before_mutation() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_207);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let spells = regular_spell_store(
        [(71_207, 0)],
        [regular_effect(
            1,
            71_207,
            0,
            0,
            APPLY_AURA,
            DUMMY_AURA as i16,
            [0; 4],
        )],
        [],
    );
    add_aura(&mut player, 71_207, 4, 7, 1);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &spells,
            &ServersideSpellStoreLikeCpp::default(),
            Some(&class_options([])),
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::DifficultyCatalog(
            wow_data::spell::DataSpellCatalogResolutionError::DifficultyUnavailable
        ))
    );
    assert_unchanged_after_preflight_error(&player, &[99_207], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn category_difficulty_cycle_fails_before_list_clear_or_rng() {
    let mut player = player_with_passive(false);
    set_history_loaded(&mut player, true);
    player.add_self_res_spell(99_212);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let spells = regular_spell_store([(ACTIVE, 4)], [], [category_row(1, ACTIVE, 0, 77)]);
    let cyclic = DifficultyStore::from_entries([DifficultyEntry {
        id: 4,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: 4,
        toggle_difficulty_id: 0,
    }]);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            4,
            &spells,
            &ServersideSpellStoreLikeCpp::default(),
            None,
            Some(&cyclic),
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::CategoryMetadata(
            wow_data::SpellCategoryReadError::Catalog(
                wow_data::spell::DataSpellCatalogResolutionError::DifficultyCycle
            )
        ))
    );
    assert_unchanged_after_preflight_error(&player, &[99_212], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn missing_target_key_fails_before_list_clear_or_rng() {
    let mut player = player_with_passive(false);
    player.add_self_res_spell(99_208);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            3,
            &SpellStore::new(),
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::SpellInfoKeyUnavailable {
            spell_id: ACTIVE,
            requested_difficulty: 3,
        })
    );
    assert_unchanged_after_preflight_error(&player, &[99_208], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn missing_history_fails_before_list_clear_or_rng() {
    let mut player = player_with_passive(false);
    player.add_self_res_spell(99_209);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(&mut serverside, ACTIVE, 0, 0, 0, [0; 4], Vec::new());
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &serverside,
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::SpellHistoryUnavailable)
    );
    assert_unchanged_after_preflight_error(&player, &[99_209], &before_mask);
    assert_eq!(rolls, 0);
}

#[test]
fn missing_category_provenance_fails_before_list_clear_or_rng() {
    let mut player = player_with_passive(false);
    set_history_loaded(&mut player, true);
    player.add_self_res_spell(99_210);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
    let spells = wow_world_test_support::spell_catalog::canonical_spell_store([(ACTIVE, 3)]);
    let mut rolls = 0;

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            3,
            &spells,
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| {
                rolls += 1;
                true
            },
        ),
        Err(SelfResurrectionError::CategoryMetadata(
            wow_data::SpellCategoryReadError::MetadataUnavailable
        ))
    );
    assert_unchanged_after_preflight_error(&player, &[99_210], &before_mask);
    assert_eq!(rolls, 0);
}
