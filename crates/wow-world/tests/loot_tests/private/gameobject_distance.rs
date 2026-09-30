//! Preserved gameobject loot application scenarios.
use super::recovery_support::*;
use std::collections::HashMap;
use std::sync::Mutex;
use wow_data::{
    SpellEffectInfo, SpellInfo, SpellMiscEntry, SpellMiscStore, SpellRangeEntry, SpellRangeStore,
    SpellStore,
};
use wow_entities::GAMEOBJECT_TYPE_GOOBER;
use wow_loot::{
    LootStore, LootStoreItem, LootStoreKind, LootStores, LootTemplateRow, loot_is_looted_like_cpp,
};
use wow_world::session::mailbox::{
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
use wow_world::test_fixtures::loot::*;

#[test]
fn gameobject_loot_distance_uses_display_box_when_db2_exists_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_041);
    session.set_player_guid(Some(player_guid));
    record_represented_gameobject_runtime_state_for_test(
        &mut session,
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_loot_display_model_for_test(
        &mut session,
        gameobject_guid,
        77,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );
    session.set_gameobject_display_info_store(Arc::new(
        wow_data::GameObjectDisplayInfoStore::from_entries([
            wow_data::GameObjectDisplayInfoEntry {
                id: 77,
                model_name: "test".to_string(),
                geo_box_min: wow_data::Db2Pos3 {
                    x: -2.0,
                    y: -1.0,
                    z: -0.5,
                },
                geo_box_max: wow_data::Db2Pos3 {
                    x: 2.0,
                    y: 1.0,
                    z: 0.5,
                },
                file_data_id: 0,
                object_effect_package_id: 0,
                override_loot_effect_scale: 0.0,
                override_name_scale: 0.0,
            },
        ]),
    ));

    session.set_player_position_like_cpp(Position::xyz(6.9, 0.0, 0.0));
    assert!(loot_gameobject_can_store_for_test(
        &session,
        gameobject_guid,
        player_guid
    ));

    session.set_player_position_like_cpp(Position::xyz(7.1, 0.0, 0.0));
    assert!(!loot_gameobject_can_store_for_test(
        &session,
        gameobject_guid,
        player_guid
    ));
}

#[test]
fn gameobject_loot_distance_uses_spell_lock_range_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_042);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::xyz(11.0, 0.0, 0.0));
    record_represented_gameobject_runtime_state_for_test(
        &mut session,
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_loot_lock_for_test(&mut session, gameobject_guid, 501);
    session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
        wow_data::LockEntry {
            id: 501,
            index: [7001, 0, 0, 0, 0, 0, 0, 0],
            skill: [0; wow_data::lock::MAX_LOCK_CASE],
            lock_type: [3, 0, 0, 0, 0, 0, 0, 0],
            action: [0; wow_data::lock::MAX_LOCK_CASE],
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        7001,
        SpellInfo {
            spell_id: 7001,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 7001,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 77,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 7001,
    }])));
    session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
        id: 77,
        display_name: "lock".to_string(),
        display_name_short: "lock".to_string(),
        flags: 0,
        range_min: [0.0, 0.0],
        range_max: [12.0, 12.0],
    }])));

    assert!(loot_gameobject_can_store_for_test(
        &session,
        gameobject_guid,
        player_guid
    ));

    session.set_player_position_like_cpp(Position::xyz(12.1, 0.0, 0.0));
    assert!(!loot_gameobject_can_store_for_test(
        &session,
        gameobject_guid,
        player_guid
    ));
}

#[test]
fn gameobject_loot_distance_uses_known_open_lock_skill_spell_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(19_043);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::xyz(8.0, 0.0, 0.0));
    prepare_money_player_residence_for_test(&mut session);
    set_loot_lock_spells_for_test(&mut session, vec![8001]);
    record_represented_gameobject_runtime_state_for_test(
        &mut session,
        0,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_loot_lock_for_test(&mut session, gameobject_guid, 502);
    session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
        wow_data::LockEntry {
            id: 502,
            index: [333, 0, 0, 0, 0, 0, 0, 0],
            skill: [50, 0, 0, 0, 0, 0, 0, 0],
            lock_type: [2, 0, 0, 0, 0, 0, 0, 0],
            action: [0; wow_data::lock::MAX_LOCK_CASE],
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        8001,
        SpellInfo {
            spell_id: 8001,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 33,
            effect_base_points: 75,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                effect: 33,
                effect_aura: 0,
                effect_base_points: 75,
                effect_misc_value_1: 333,
                effect_misc_value_2: 0,
                effect_radius_index_1: 0,
                chain_targets: 0,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 8001,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 88,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 8001,
    }])));
    session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
        id: 88,
        display_name: "skill".to_string(),
        display_name_short: "skill".to_string(),
        flags: 0,
        range_min: [0.0, 0.0],
        range_max: [9.0, 9.0],
    }])));

    assert!(loot_gameobject_can_store_for_test(
        &session,
        gameobject_guid,
        player_guid
    ));
}
