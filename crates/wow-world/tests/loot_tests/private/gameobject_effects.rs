//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[tokio::test]
async fn represented_gameobject_chest_first_generation_records_use_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_002);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 55,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 0,
        triggered_event_id: 777,
        linked_trap_entry: 888,
        ..Default::default()
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source)
        .await;
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source)
        .await;

    assert_eq!(
        gameobject_loot_effects_for_test(&session),
        vec![
            LootUseEffect::trigger_event(gameobject_guid, player_guid, 777),
            LootUseEffect::trigger_trap(gameobject_guid, player_guid, 888),
        ]
    );
}

#[tokio::test]
async fn represented_gameobject_chest_no_loot_unique_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_005);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 0,
        triggered_event_id: 901,
        linked_trap_entry: 902,
        ..Default::default()
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source)
        .await;
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source)
        .await;

    assert_eq!(
        gameobject_loot_effects_for_test(&session),
        vec![
            LootUseEffect::trigger_event(gameobject_guid, player_guid, 901),
            LootUseEffect::trigger_trap(gameobject_guid, player_guid, 902),
        ]
    );
}

#[tokio::test]
async fn represented_gameobject_chest_push_unique_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_004);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 99,
        triggered_event_id: 321,
        linked_trap_entry: 654,
        ..Default::default()
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source)
        .await;
    open_gameobject_loot_cycle_for_test(&mut session, gameobject_guid, source)
        .await;

    assert_eq!(
        gameobject_loot_effects_for_test(&session),
        vec![
            LootUseEffect::trigger_event(gameobject_guid, player_guid, 321),
            LootUseEffect::trigger_trap(gameobject_guid, player_guid, 654),
        ]
    );
}

#[tokio::test]
async fn represented_fishing_hole_updates_catch_criteria_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_005);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    prepare_money_player_residence_for_test(&mut session);
    open_fishing_hole_cycle_for_test(&mut session, gameobject_guid, 190_000, 123)
        .await;

    assert_eq!(
        gameobject_loot_effects_for_test(&session),
        vec![
            LootUseEffect::fishing_catch(gameobject_guid, player_guid, 190_000)
        ]
    );
}

#[tokio::test]
async fn represented_gathering_node_first_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_003);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 0,
        triggered_event_id: 123,
        xp_difficulty: 0,
        spell_id: 0,
        max_loots: 10,
        linked_trap_entry: 456,
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gathering_loot_cycle_for_test(&mut session, gameobject_guid, 190_003, source)
        .await;
    open_gathering_loot_cycle_for_test(&mut session, gameobject_guid, 190_003, source)
        .await;

    assert_eq!(
        gameobject_loot_effects_for_test(&session),
        vec![
            LootUseEffect::trigger_event(gameobject_guid, player_guid, 123),
            LootUseEffect::trigger_trap(gameobject_guid, player_guid, 456),
        ]
    );
}

#[tokio::test]
async fn represented_gathering_node_runtime_state_matches_cpp_side_effects() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_007);
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 15,
        triggered_event_id: 0,
        xp_difficulty: 0,
        spell_id: 777,
        max_loots: 1,
        linked_trap_entry: 0,
    };

    prepare_money_player_residence_for_test(&mut session);
    open_gathering_loot_cycle_for_test(&mut session, gameobject_guid, 190_007, source)
        .await;
    open_gathering_loot_cycle_for_test(&mut session, gameobject_guid, 190_007, source)
        .await;

    let state = loot_gameobject_state_for_test(&session, gameobject_guid)
        .expect("represented gathering use records GO state");
    assert_eq!(state.personal_loot_uses(), 1);
    assert_eq!(state.go_state(), Some(GoState::Active));
    assert_eq!(
        state.dynamic_flags() & GO_DYNFLAG_LO_NO_INTERACT,
        GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.loot_state(), Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid(), player_guid);
    assert_eq!(state.despawn_delay_secs(), Some(15));
    assert!(state.despawn_delay_until().is_some());
    assert_eq!(
        gameobject_loot_effects_for_test(&session),
        vec![
            LootUseEffect::outdoor_spell(gameobject_guid, player_guid, 190_007, 777, GAMEOBJECT_TYPE_GATHERING_NODE, 0, false),
            LootUseEffect::post_use_spell(gameobject_guid, player_guid, player_guid, 777, false, 0),
        ]
    );
}

