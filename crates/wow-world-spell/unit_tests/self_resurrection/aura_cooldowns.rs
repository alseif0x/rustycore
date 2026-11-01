use super::fixtures::*;
use crate::self_resurrection::{SelfResurrectionError, initialize_self_resurrection_spells};
use wow_data::{ServersideSpellStoreLikeCpp, SpellCategoryReadError, SpellStore};
use wow_entities::PlayerSpellLoadState;

#[test]
fn cooldown_383_generic_family_allows_reincarnation_unconditionally() {
    let mut player = player_with_passive(false);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_100,
        0,
        0,
        0,
        [0; 4],
        vec![server_effect(
            0,
            APPLY_AURA as i32,
            IGNORE_COOLDOWN_AURA,
            [u32::MAX as i32; 4],
        )],
    );
    insert_server_spell(&mut serverside, ACTIVE, 0, 0, 33, [0; 4], Vec::new());
    add_aura(&mut player, 71_100, 0, 0, 1);

    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |_| panic!("ignore-cooldown aura bypasses history query"),
    )
    .unwrap();

    assert_eq!(player.self_res_spells(), [21_169]);
}

#[test]
fn cooldown_383_same_family_zero_mask_allows_reincarnation() {
    let mut player = player_with_passive(false);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_101,
        0,
        0,
        31,
        [0; 4],
        vec![server_effect(
            0,
            APPLY_AURA as i32,
            IGNORE_COOLDOWN_AURA,
            [0; 4],
        )],
    );
    insert_server_spell(&mut serverside, ACTIVE, 0, 0, 31, [0; 4], Vec::new());
    add_aura(&mut player, 71_101, 0, 0, 1);

    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |_| panic!("same-family zero mask bypasses history query"),
    )
    .unwrap();

    assert_eq!(player.self_res_spells(), [21_169]);
}

#[test]
fn cooldown_383_family_mismatch_does_not_ignore_cooldown() {
    let mut player = player_with_passive(false);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_102,
        0,
        0,
        30,
        [1, 0, 0, 0],
        vec![server_effect(
            0,
            APPLY_AURA as i32,
            IGNORE_COOLDOWN_AURA,
            [1, 0, 0, 0],
        )],
    );
    insert_server_spell(&mut serverside, ACTIVE, 0, 0, 31, [1, 0, 0, 0], Vec::new());
    add_aura(&mut player, 71_102, 0, 0, 1);

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &serverside,
            None,
            None,
            |_| panic!("a mismatch reaches history before any random roll"),
        ),
        Err(SelfResurrectionError::SpellHistoryUnavailable)
    );
}

#[test]
fn cooldown_383_uses_all_four_class_mask_words_and_full_width_server_family() {
    let family = 0x1_0001;
    let mut player = player_with_passive(false);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_103,
        0,
        0,
        family,
        [0, 0, 0, 0x20],
        vec![server_effect(
            0,
            APPLY_AURA as i32,
            IGNORE_COOLDOWN_AURA,
            [0, 0, 0, 0x20],
        )],
    );
    insert_server_spell(
        &mut serverside,
        ACTIVE,
        0,
        0,
        family,
        [0, 0, 0, 0x20],
        Vec::new(),
    );
    add_aura(&mut player, 71_103, 0, 0, 1);

    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |_| panic!("fourth-word overlap suppresses cooldown query"),
    )
    .unwrap();
    assert_eq!(player.self_res_spells(), [21_169]);

    let mut player = player_with_passive(false);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_104,
        0,
        0,
        family,
        [0, 0, 0, 0x20],
        vec![server_effect(
            0,
            APPLY_AURA as i32,
            IGNORE_COOLDOWN_AURA,
            [0, 0, 0, 0x40],
        )],
    );
    insert_server_spell(
        &mut serverside,
        ACTIVE,
        0,
        0,
        family,
        [0, 0, 0, 0x20],
        Vec::new(),
    );
    add_aura(&mut player, 71_104, 0, 0, 1);
    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &serverside,
            None,
            None,
            |_| panic!("non-overlap reaches history"),
        ),
        Err(SelfResurrectionError::SpellHistoryUnavailable)
    );
}

#[test]
fn aura_uses_its_raw_difficulty_and_target_uses_current_map_difficulty() {
    let mut player = player_with_passive(false);
    set_history_loaded(&mut player, true);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        71_105,
        3,
        0,
        44,
        [0; 4],
        vec![server_effect(
            0,
            APPLY_AURA as i32,
            IGNORE_COOLDOWN_AURA,
            [0; 4],
        )],
    );
    insert_server_spell(
        &mut serverside,
        71_105,
        4,
        0,
        44,
        [0; 4],
        vec![server_effect(0, APPLY_AURA as i32, 31, [0; 4])],
    );
    insert_server_spell(&mut serverside, ACTIVE, 3, 0, 45, [0; 4], Vec::new());
    insert_server_spell(&mut serverside, ACTIVE, 4, 0, 44, [0; 4], Vec::new());
    add_aura(&mut player, 71_105, 3, 0, 1);

    initialize_self_resurrection_spells(
        &mut player,
        4,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |_| panic!("raw-difficulty Aura383 is effective on current-difficulty target"),
    )
    .unwrap();
    assert_eq!(player.self_res_spells(), [21_169]);
}

#[test]
fn direct_cooldown_membership_precedes_category_metadata_and_ignores_expiry() {
    for (on_hold, end_ms) in [(false, 0), (true, u64::MAX)] {
        let mut player = player_with_passive(false);
        set_history_loaded(&mut player, true);
        insert_direct_cooldown(&mut player, on_hold, end_ms);
        let spells = wow_world_test_support::spell_catalog::canonical_spell_store([(ACTIVE, 3)]);
        assert!(spells.get(ACTIVE as i32).is_none());

        initialize_self_resurrection_spells(
            &mut player,
            3,
            &spells,
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| panic!("direct spell cooldown prevents resurrection"),
        )
        .unwrap();
        assert!(player.self_res_spells().is_empty());
    }
}

#[test]
fn category_cooldown_membership_uses_the_exact_target_key_category() {
    let mut player = player_with_passive(false);
    set_history_loaded(&mut player, true);
    player
        .unit_mut()
        .subsystems_mut()
        .spells
        .history
        .category_cooldowns
        .insert(77, ACTIVE);
    let spells = regular_spell_store([(ACTIVE, 3)], [], [category_row(1, ACTIVE, 3, 77)]);
    assert!(spells.get(ACTIVE as i32).is_none());

    initialize_self_resurrection_spells(
        &mut player,
        3,
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        None,
        None,
        |_| panic!("category cooldown prevents resurrection"),
    )
    .unwrap();
    assert!(player.self_res_spells().is_empty());
}

#[test]
fn known_empty_categories_allow_reincarnation_without_a_difficulty_graph() {
    let mut player = player_with_passive(false);
    set_history_loaded(&mut player, true);
    let spells = regular_spell_store([(ACTIVE, 3)], [], []);
    assert!(spells.get(ACTIVE as i32).is_none());

    initialize_self_resurrection_spells(
        &mut player,
        3,
        &spells,
        &ServersideSpellStoreLikeCpp::default(),
        None,
        None,
        |_| false,
    )
    .unwrap();
    assert_eq!(player.self_res_spells(), [21_169]);
}

#[test]
fn missing_category_provenance_errors_only_when_category_lookup_is_reached() {
    let mut player = player_with_passive(false);
    set_history_loaded(&mut player, true);
    let spells = wow_world_test_support::spell_catalog::canonical_spell_store([(ACTIVE, 3)]);

    assert_eq!(
        initialize_self_resurrection_spells(
            &mut player,
            3,
            &spells,
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| panic!("metadata failure precedes RNG"),
        ),
        Err(SelfResurrectionError::CategoryMetadata(
            SpellCategoryReadError::MetadataUnavailable
        ))
    );
}

#[test]
fn loaded_but_incomplete_spellbook_rejects_absent_reincarnation_row() {
    let mut player = player_with_rows([], false);
    player.add_self_res_spell(99_110);
    let before_mask = player.active_data().self_res_spells_update_mask.clone();
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
        Err(SelfResurrectionError::SpellbookUnavailable {
            spell_id: PASSIVE as u32
        })
    );
    assert_eq!(player.self_res_spells(), [99_110]);
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        before_mask
    );
    assert_eq!(rolls, 0);
}

#[test]
fn a_relevant_passive_row_proves_state_without_complete_spellbook() {
    let mut player = player_with_rows(
        [spell_row(
            PASSIVE,
            PlayerSpellLoadState::Unchanged,
            false,
            false,
        )],
        false,
    );
    set_history_loaded(&mut player, true);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(&mut serverside, ACTIVE, 0, 0, 0, [0; 4], Vec::new());

    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |_| false,
    )
    .unwrap();
    assert_eq!(player.self_res_spells(), [21_169]);
}
