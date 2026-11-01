use super::fixtures::*;
use crate::self_resurrection::initialize_self_resurrection_spells;

use wow_data::{ServersideSpellStoreLikeCpp, SpellStore};
use wow_entities::PlayerSpellLoadState;

#[test]
fn rebuild_clears_then_appends_native_list_in_fixed_priority_order() {
    let mut player = player_with_passive(false);
    player.add_self_res_spell(99_001);
    set_history_loaded(&mut player, true);

    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        90_001,
        0,
        0,
        5,
        [0, 0x0100_0000, 0, 0],
        vec![server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4])],
    );
    insert_server_spell(
        &mut serverside,
        TWISTING_NETHER,
        0,
        0,
        0,
        [0; 4],
        vec![server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4])],
    );
    insert_server_spell(&mut serverside, ACTIVE, 0, 0, 0, [0; 4], Vec::new());
    add_aura(&mut player, 90_001, 0, 8, 1);
    add_aura(&mut player, TWISTING_NETHER as i32, 0, 2, 1);

    let mut chances = Vec::new();
    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |chance| {
            chances.push(chance);
            true
        },
    )
    .unwrap();

    assert_eq!(chances, [10]);
    assert_eq!(player.self_res_spells(), [3026, 23_700, 21_169]);
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![0b111])
    );
}

#[test]
fn every_duplicate_twisting_nether_effect_rolls_after_success() {
    let mut player = player_with_rows([], true);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        TWISTING_NETHER,
        0,
        0,
        0,
        [0; 4],
        vec![
            server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4]),
            server_effect(1, APPLY_AURA as i32, DUMMY_AURA, [0; 4]),
        ],
    );
    add_aura(&mut player, TWISTING_NETHER as i32, 0, 0, 0b11);

    let mut chances = Vec::new();
    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |chance| {
            chances.push(chance);
            chances.len() == 1
        },
    )
    .unwrap();

    assert_eq!(chances, [10, 10]);
    assert_eq!(player.self_res_spells(), [23_700]);
}

#[test]
fn soulstone_branch_precedes_twisting_nether_roll_for_the_same_effect() {
    let mut player = player_with_rows([], true);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        TWISTING_NETHER,
        0,
        0,
        5,
        [0, 0x0100_0000, 0, 0],
        vec![server_effect(0, APPLY_AURA as i32, DUMMY_AURA, [0; 4])],
    );
    add_aura(&mut player, TWISTING_NETHER as i32, 0, 0, 1);

    let mut rolls = 0;
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
    )
    .unwrap();

    assert_eq!(rolls, 0);
    assert_eq!(player.self_res_spells(), [3026]);
}

#[test]
fn zero_mask_unknown_spell_is_skipped_and_empty_result_still_clears_native_field() {
    let mut player = player_with_rows([], true);
    player.add_self_res_spell(99_002);
    add_aura(&mut player, 0xF123_4567u32 as i32, 247, 1, 0);

    let mut rolls = 0;
    initialize_self_resurrection_spells(
        &mut player,
        199,
        &SpellStore::new(),
        &ServersideSpellStoreLikeCpp::default(),
        None,
        None,
        |_| {
            rolls += 1;
            true
        },
    )
    .unwrap();

    assert!(player.self_res_spells().is_empty());
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(Vec::new())
    );
    assert_eq!(rolls, 0);
}

#[test]
fn persistent_area_dummy_effect_is_a_native_aura_and_rolls_twisting_nether() {
    let mut player = player_with_rows([], true);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        TWISTING_NETHER,
        0,
        0,
        0,
        [0; 4],
        vec![server_effect(
            0,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_PERSISTENT_AREA_AURA as i32,
            DUMMY_AURA,
            [0; 4],
        )],
    );
    add_aura(&mut player, TWISTING_NETHER as i32, 0, 0, 1);

    let mut chances = Vec::new();
    initialize_self_resurrection_spells(
        &mut player,
        0,
        &SpellStore::new(),
        &serverside,
        None,
        None,
        |chance| {
            chances.push(chance);
            true
        },
    )
    .unwrap();

    assert_eq!(chances, [10]);
    assert_eq!(player.self_res_spells(), [23_700]);
}

#[test]
fn inactive_dummy_effect_bit_does_not_roll() {
    let mut player = player_with_rows([], true);
    let mut serverside = ServersideSpellStoreLikeCpp::default();
    insert_server_spell(
        &mut serverside,
        TWISTING_NETHER,
        0,
        0,
        0,
        [0; 4],
        vec![
            server_effect(0, APPLY_AURA as i32, 31, [0; 4]),
            server_effect(1, APPLY_AURA as i32, DUMMY_AURA, [0; 4]),
        ],
    );
    add_aura(&mut player, TWISTING_NETHER as i32, 0, 0, 1);

    let mut rolls = 0;
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
    )
    .unwrap();

    assert_eq!(rolls, 0);
    assert!(player.self_res_spells().is_empty());
}

#[test]
fn inactive_reincarnation_spell_row_is_known_even_when_not_active() {
    let mut player = player_with_passive(false);
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

#[test]
fn removed_or_disabled_reincarnation_row_does_not_resolve_as_known() {
    for row in [
        spell_row(PASSIVE, PlayerSpellLoadState::Removed, true, false),
        spell_row(PASSIVE, PlayerSpellLoadState::Changed, true, true),
    ] {
        let mut player = player_with_rows([row], false);
        player.add_self_res_spell(99_003);
        initialize_self_resurrection_spells(
            &mut player,
            0,
            &SpellStore::new(),
            &ServersideSpellStoreLikeCpp::default(),
            None,
            None,
            |_| panic!("ineligible spell rows do not roll"),
        )
        .unwrap();

        assert!(player.self_res_spells().is_empty());
    }
}
