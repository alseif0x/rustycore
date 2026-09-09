//! GameObject template, loot and runtime state regression scenarios, part 2 of 3.
//!
//! Moved out of the game_object.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn capture_point_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 60_000;
    data[4] = 401;
    data[5] = 501;
    data[6] = 601;
    data[7] = 701;
    data[8] = 801;
    data[9] = 901;
    data[10] = 123;
    data[11] = 456;
    data[12] = 1200;
    data[13] = 1300;
    data[14] = 789;
    data[15] = 1500;
    data[16] = 1600;
    data[17] = 1700;
    data[18] = 1800;
    data[19] = 1900;
    data[20] = 2000;
    data[21] = 2100;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CAPTURE_POINT, data)
            .capture_point_use_source_like_cpp(),
        Some(CapturePointUseSource {
            capture_time_ms: 60_000,
            assault_broadcast_horde: 401,
            capture_broadcast_horde: 501,
            defended_broadcast_horde: 601,
            assault_broadcast_alliance: 701,
            capture_broadcast_alliance: 801,
            defended_broadcast_alliance: 901,
            world_state_id: 123,
            contested_event_horde: 456,
            capture_event_horde: 1200,
            defended_event_horde: 1300,
            contested_event_alliance: 789,
            capture_event_alliance: 1500,
            defended_event_alliance: 1600,
            spell_visual_ids: [1700, 1800, 1900, 2000, 2100],
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
            .capture_point_use_source_like_cpp(),
        None
    );
}

#[test]
fn battleground_flag_use_sources_use_cpp_data_indices() {
    let mut stand = [0; MAX_GAMEOBJECT_DATA];
    stand[1] = 111;
    stand[3] = 333;
    stand[4] = 444;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FLAGSTAND, stand)
            .flag_stand_use_source_like_cpp(),
        Some(FlagStandUseSource {
            pickup_spell_id: 111,
            return_aura_id: 333,
            return_spell_id: 444,
        })
    );

    let mut drop = [0; MAX_GAMEOBJECT_DATA];
    drop[1] = 222;
    drop[2] = 333;
    drop[6] = 666;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FLAGDROP, drop).flag_drop_use_source_like_cpp(),
        Some(FlagDropUseSource {
            event_id: 222,
            pickup_spell_id: 333,
            expire_duration_ms: 666,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, drop).flag_drop_use_source_like_cpp(),
        None
    );
}

#[test]
fn questgiver_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[3] = 42;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_QUESTGIVER, data)
            .questgiver_use_source_like_cpp(),
        Some(QuestgiverUseSource { gossip_id: 42 })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).questgiver_use_source_like_cpp(),
        None
    );
}

#[test]
fn ritual_and_meeting_stone_use_sources_use_cpp_data_indices() {
    let mut ritual = [0; MAX_GAMEOBJECT_DATA];
    ritual[0] = 3;
    ritual[1] = 62330;
    ritual[2] = 111;
    ritual[3] = 1;
    ritual[4] = 222;
    ritual[5] = 2;
    ritual[6] = 1;
    ritual[7] = 1;
    ritual[10] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_RITUAL, ritual).ritual_use_source_like_cpp(),
        Some(RitualUseSource {
            casters_required: 3,
            spell_id: 62330,
            anim_spell_id: 111,
            persistent: true,
            caster_target_spell_id: 222,
            caster_target_spell_targets: 2,
            casters_grouped: true,
            no_target_check: true,
            allow_unfriendly_cross_faction_party: true,
        })
    );

    let mut meeting_stone = [0; MAX_GAMEOBJECT_DATA];
    meeting_stone[2] = 345;
    meeting_stone[4] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MEETINGSTONE, meeting_stone)
            .meeting_stone_use_source_like_cpp(),
        Some(MeetingStoneUseSource {
            area_id: 345,
            prevent_unfriendly_outside_instances: true,
            content_tuning_id: 0,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, ritual).ritual_use_source_like_cpp(),
        None
    );
}

#[test]
fn new_flag_use_sources_use_cpp_data_indices() {
    let mut flag = [0; MAX_GAMEOBJECT_DATA];
    flag[1] = 111;
    flag[7] = 777;
    flag[8] = 888;
    flag[9] = 999;
    flag[10] = u32::MAX;
    flag[11] = 1111;
    flag[12] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_NEW_FLAG, flag).new_flag_use_source_like_cpp(),
        Some(NewFlagUseSource {
            pickup_spell_id: 111,
            expire_duration_ms: 777,
            respawn_time_ms: 888,
            flag_drop_entry: 999,
            exclusive_category: -1,
            world_state_id: 1111,
            return_on_defender_interact: true,
        })
    );

    let mut drop = [0; MAX_GAMEOBJECT_DATA];
    drop[1] = 222;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_NEW_FLAG_DROP, drop)
            .new_flag_drop_use_source_like_cpp(),
        Some(NewFlagDropUseSource {
            spawn_vignette_id: 222,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, drop).new_flag_use_source_like_cpp(),
        None
    );
}

#[test]
fn spellcaster_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 1234;
    data[1] = 7;
    data[2] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELLCASTER, data)
            .spellcaster_use_source_like_cpp(),
        Some(SpellcasterUseSource {
            spell_id: 1234,
            charges: 7,
            party_only: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).spellcaster_use_source_like_cpp(),
        None
    );
}

#[test]
fn spell_focus_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_SPELL_FOCUS_TYPE] = 181;
    data[GAMEOBJECT_DATA_SPELL_FOCUS_RADIUS] = 12;
    data[GAMEOBJECT_DATA_SPELL_FOCUS_LINKED_TRAP] = 987;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELL_FOCUS, data)
            .spell_focus_use_source_like_cpp(),
        Some(SpellFocusUseSource {
            focus_type: 181,
            radius: 12,
            linked_trap_entry: 987,
        })
    );

    let mut ui_link_data = [0; MAX_GAMEOBJECT_DATA];
    ui_link_data[GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_TYPE] = 182;
    ui_link_data[GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_RADIUS] = 34;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_UI_LINK, ui_link_data)
            .spell_focus_use_source_like_cpp(),
        Some(SpellFocusUseSource {
            focus_type: 182,
            radius: 34,
            linked_trap_entry: 0,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).spell_focus_use_source_like_cpp(),
        None
    );
}

#[test]
fn guard_post_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 4321;
    data[1] = 5;
    data[2] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GUARDPOST, data)
            .guard_post_use_source_like_cpp(),
        Some(GuardPostUseSource {
            creature_id: 4321,
            charges: 5,
            prefer_only_if_in_line_of_sight: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).guard_post_use_source_like_cpp(),
        None
    );
}

#[test]
fn spell_focus_linked_trap_uses_cpp_data_index() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[2] = 987;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELL_FOCUS, data)
            .spell_focus_linked_trap_like_cpp(),
        987
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).spell_focus_linked_trap_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_linked_gameobject_entry_dispatches_cpp_sources() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_BUTTON_LINKED_TRAP] = 103;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_BUTTON, data)
            .get_linked_gameobject_entry_like_cpp(),
        103
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[2] = 802;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELL_FOCUS, data)
            .get_linked_gameobject_entry_like_cpp(),
        802
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[12] = 1012;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, data)
            .get_linked_gameobject_entry_like_cpp(),
        1012
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LINKED_TRAP] = 307;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
            .get_linked_gameobject_entry_like_cpp(),
        307
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP] = 5020;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GATHERING_NODE, data)
            .get_linked_gameobject_entry_like_cpp(),
        5020
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TRAP, data)
            .get_linked_gameobject_entry_like_cpp(),
        0
    );
}

#[test]
fn gameobject_linked_trap_guid_defaults_and_can_be_cleared_like_cpp() {
    let mut go = GameObject::new();
    assert_eq!(go.linked_trap_guid_like_cpp(), ObjectGuid::EMPTY);
    let trap_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 1, 9002, 2);

    go.set_linked_trap_like_cpp(trap_guid);
    assert_eq!(go.linked_trap_guid_like_cpp(), trap_guid);

    go.set_linked_trap_like_cpp(ObjectGuid::EMPTY);
    assert_eq!(go.linked_trap_guid_like_cpp(), ObjectGuid::EMPTY);
}

#[test]
fn camera_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[1] = 1234;
    data[2] = 55;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CAMERA, data).camera_use_source_like_cpp(),
        Some(CameraUseSource {
            cinematic_id: 1234,
            event_id: 55,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).camera_use_source_like_cpp(),
        None
    );
}

#[test]
fn goober_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 99;
    data[1] = 101;
    data[2] = 202;
    data[3] = 303;
    data[4] = 4;
    data[5] = 1;
    data[7] = 707;
    data[10] = 1010;
    data[12] = 1212;
    data[19] = 1919;
    data[20] = 1;
    data[23] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, data).goober_use_source_like_cpp(),
        Some(GooberUseSource {
            lock_id: 99,
            quest_id: 101,
            event_id: 202,
            auto_close_ms: 303,
            custom_anim: 4,
            consumable: true,
            page_id: 707,
            spell_id: 1010,
            linked_trap_entry: 1212,
            gossip_id: 1919,
            allow_multi_interact: true,
            player_cast: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).goober_use_source_like_cpp(),
        None
    );
}

#[test]
fn chest_loot_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 10;
    data[GAMEOBJECT_DATA_CHEST_RESTOCK_TIME] = 60;
    data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] = 1;
    data[GAMEOBJECT_DATA_CHEST_TRIGGERED_EVENT] = 40;
    data[GAMEOBJECT_DATA_CHEST_LINKED_TRAP] = 50;
    data[GAMEOBJECT_DATA_CHEST_QUEST_ID] = 9999;
    data[GAMEOBJECT_DATA_CHEST_USE_GROUP_LOOT_RULES] = 1;
    data[GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER] = 1234;
    data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT] = 20;
    data[GAMEOBJECT_DATA_CHEST_PUSH_LOOT] = 30;

    let source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");

    assert_eq!(
        source,
        GameObjectLootSource {
            loot_id: 10,
            use_group_loot_rules: true,
            dungeon_encounter_id: 1234,
            personal_loot_id: 20,
            push_loot_id: 30,
            triggered_event_id: 40,
            linked_trap_entry: 50,
            chest_restock_time_secs: 60,
            chest_consumable: true,
            chest_quest_id: 9999,
        }
    );
    assert!(!source.is_empty());
    assert_eq!(source.open_loot_id_like_cpp(), 10);
    assert!(source.has_open_loot_like_cpp());
    assert!(!source.uses_personal_loot_like_cpp());
    assert!(!source.is_personal_encounter_loot_like_cpp());
    assert!(!source.should_autostore_push_loot_like_cpp());

    // Verify index 8 is read from the correct data slot: zero it and confirm field resets.
    data[GAMEOBJECT_DATA_CHEST_QUEST_ID] = 0;
    let no_quest_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert_eq!(no_quest_source.chest_quest_id, 0);

    data[GAMEOBJECT_DATA_CHEST_LOOT] = 0;
    data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT] = 0;
    let push_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert!(!push_source.is_empty());
    assert!(!push_source.has_open_loot_like_cpp());
    assert!(!push_source.uses_personal_loot_like_cpp());
    assert!(!push_source.is_personal_encounter_loot_like_cpp());
    assert!(push_source.should_autostore_push_loot_like_cpp());

    data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT] = 25;
    let personal_encounter_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert_eq!(personal_encounter_source.open_loot_id_like_cpp(), 25);
    assert!(personal_encounter_source.has_open_loot_like_cpp());
    assert!(personal_encounter_source.uses_personal_loot_like_cpp());
    assert!(personal_encounter_source.is_personal_encounter_loot_like_cpp());

    data[GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER] = 0;
    let personal_non_encounter_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert!(personal_non_encounter_source.uses_personal_loot_like_cpp());
    assert!(!personal_non_encounter_source.is_personal_encounter_loot_like_cpp());
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FISHING_HOLE, data)
            .chest_loot_source_like_cpp(),
        None
    );
}

#[test]
fn gathering_node_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 10;
    data[GAMEOBJECT_DATA_GATHERING_NODE_DESPAWN_DELAY] = 15;
    data[GAMEOBJECT_DATA_GATHERING_NODE_TRIGGERED_EVENT] = 20;
    data[GAMEOBJECT_DATA_GATHERING_NODE_XP_DIFFICULTY] = 5;
    data[GAMEOBJECT_DATA_GATHERING_NODE_SPELL] = 30;
    data[GAMEOBJECT_DATA_GATHERING_NODE_MAX_LOOTS] = 3;
    data[GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP] = 40;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GATHERING_NODE, data)
            .gathering_node_use_source_like_cpp(),
        Some(GatheringNodeUseSource {
            loot_id: 10,
            despawn_delay_secs: 15,
            triggered_event_id: 20,
            xp_difficulty: 5,
            spell_id: 30,
            max_loots: 3,
            linked_trap_entry: 40,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
            .gathering_node_use_source_like_cpp(),
        None
    );
}

#[test]
fn gameobject_update_no_despawn_delay_stays_updated_like_cpp() {
    let mut go = GameObject::new();

    let outcome = go.update_like_cpp(40);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.despawn_delay_before_ms, 0);
    assert_eq!(outcome.despawn_delay_after_ms, 0);
    assert!(!outcome.despawn_or_unsummon_requested);
    assert!(outcome.world_update_would_run);
    assert!(outcome.ai_update_not_represented);
    assert!(outcome.go_type_impl_update_not_represented);
}

#[test]
fn gameobject_update_decrements_pending_despawn_delay_like_cpp() {
    let mut go = GameObject::new();
    assert!(go.schedule_despawn_or_unsummon_like_cpp(100, 7));

    let outcome = go.update_like_cpp(40);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.despawn_delay_before_ms, 100);
    assert_eq!(outcome.despawn_delay_after_ms, 60);
    assert_eq!(outcome.despawn_respawn_time_secs, 7);
    assert_eq!(go.despawn_delay(), 60);
    assert!(!outcome.despawn_or_unsummon_requested);
}

#[test]
fn gameobject_update_expired_despawn_delay_requests_immediate_despawn_like_cpp() {
    let mut exact = GameObject::new();
    assert!(exact.schedule_despawn_or_unsummon_like_cpp(40, 9));
    let exact_outcome = exact.update_like_cpp(40);
    assert_eq!(
        exact_outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRequested
    );
    assert_eq!(exact_outcome.despawn_delay_before_ms, 40);
    assert_eq!(exact_outcome.despawn_delay_after_ms, 0);
    assert_eq!(exact_outcome.despawn_respawn_time_secs, 9);
    assert!(exact_outcome.despawn_or_unsummon_requested);
    assert_eq!(exact.despawn_delay(), 0);
    assert_eq!(exact.despawn_respawn_time(), 9);

    let mut overshoot = GameObject::new();
    assert!(overshoot.schedule_despawn_or_unsummon_like_cpp(40, 11));
    let overshoot_outcome = overshoot.update_like_cpp(50);
    assert_eq!(
        overshoot_outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRequested
    );
    assert_eq!(overshoot_outcome.despawn_delay_before_ms, 40);
    assert_eq!(overshoot_outcome.despawn_delay_after_ms, 0);
    assert_eq!(overshoot_outcome.despawn_respawn_time_secs, 11);
    assert!(overshoot_outcome.despawn_or_unsummon_requested);
    assert_eq!(overshoot.despawn_respawn_time(), 11);
}

#[test]
fn gameobject_update_despawn_scheduler_only_shortens_pending_delay_like_cpp() {
    let mut go = GameObject::new();

    assert!(!go.schedule_despawn_or_unsummon_like_cpp(0, 3));
    assert_eq!(go.despawn_delay(), 0);
    assert_eq!(go.despawn_respawn_time(), 0);

    assert!(go.schedule_despawn_or_unsummon_like_cpp(100, 7));
    assert_eq!(go.despawn_delay(), 100);
    assert_eq!(go.despawn_respawn_time(), 7);

    assert!(!go.schedule_despawn_or_unsummon_like_cpp(150, 9));
    assert_eq!(go.despawn_delay(), 100);
    assert_eq!(go.despawn_respawn_time(), 7);

    assert!(go.schedule_despawn_or_unsummon_like_cpp(40, 11));
    assert_eq!(go.despawn_delay(), 40);
    assert_eq!(go.despawn_respawn_time(), 11);
}

#[test]
fn gameobject_owned_loot_is_looted_matches_cpp_gold_and_unlooted_count() {
    let empty = GameObjectOwnedLoot::default();
    assert_eq!(empty.gold(), 0);
    assert_eq!(empty.unlooted_count(), 0);
    assert!(empty.is_looted_like_cpp());

    let gold_only = GameObjectOwnedLoot::new(1, 0);
    assert_eq!(gold_only.gold(), 1);
    assert_eq!(gold_only.unlooted_count(), 0);
    assert!(!gold_only.is_looted_like_cpp());

    let items_only = GameObjectOwnedLoot::new(0, 1);
    assert_eq!(items_only.gold(), 0);
    assert_eq!(items_only.unlooted_count(), 1);
    assert!(!items_only.is_looted_like_cpp());

    assert!(!GameObjectOwnedLoot::new(1, 1).is_looted_like_cpp());
}

#[test]
fn gameobject_personal_authority_updates_summary_and_clear_retires_it() {
    let player = ObjectGuid::create_player(1, 77);
    let mut gameobject = GameObject::new();
    let full_loot = CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 23,
        unlooted_count: 1,
        loot_type: 9,
        dungeon_encounter_id: 0,
        loot_method: 5,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player],
        items: Vec::new(),
        looted_by_player: false,
    };
    gameobject.upsert_personal_loot_authority_like_cpp(player, full_loot, false);

    assert_eq!(
        gameobject.personal_loot_like_cpp(player),
        Some(&GameObjectOwnedLoot::new(23, 1))
    );
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .snapshot_for_player_like_cpp(player)
            .is_some()
    );
    gameobject.clear_loot_like_cpp();
    assert!(gameobject.loot_authority_like_cpp().is_retired_like_cpp());
    assert_eq!(gameobject.personal_loot_count_like_cpp(), 0);
}

#[test]
fn clear_loot_restock_starts_second_personal_generation_for_same_gameobject() {
    let player = ObjectGuid::create_player(1, 78);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.upsert_personal_loot_authority_like_cpp(
        player,
        owned_loot_fixture_like_cpp(7, 0),
        false,
    );
    let first_generation = gameobject.loot_authority_like_cpp().generation_like_cpp();

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    let authority = gameobject.loot_authority_like_cpp().clone();
    let tombstone_generation = authority.generation_like_cpp();
    let lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    assert!(gameobject.install_personal_loot_if_lifecycle_like_cpp(
        &authority,
        tombstone_generation,
        lifecycle_revision,
        player,
        owned_loot_fixture_like_cpp(19, 0),
        false,
    ));
    let second = gameobject
        .loot_authority_like_cpp()
        .snapshot_for_player_like_cpp(player)
        .expect("restocked personal pool");
    assert_eq!(second.loot.coins, 19);
    assert!(gameobject.loot_authority_like_cpp().generation_like_cpp() > first_generation);
    assert!(!gameobject.loot_authority_like_cpp().is_retired_like_cpp());
}

#[test]
fn stale_personal_generator_cannot_cross_gameobject_clear_loot_lifecycle() {
    let player = ObjectGuid::create_player(1, 79);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.upsert_personal_loot_authority_like_cpp(
        player,
        owned_loot_fixture_like_cpp(3, 0),
        false,
    );
    let stale_authority = gameobject.loot_authority_like_cpp().clone();
    let stale_generation = stale_authority.generation_like_cpp();
    let stale_lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);

    assert!(!gameobject.install_personal_loot_if_lifecycle_like_cpp(
        &stale_authority,
        stale_generation,
        stale_lifecycle_revision,
        player,
        owned_loot_fixture_like_cpp(99, 0),
        false,
    ));
    assert!(gameobject.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .snapshot_for_player_like_cpp(player)
            .is_none()
    );
}

#[test]
fn clear_loot_restock_installs_shared_generation_once_for_same_lifecycle() {
    let player = ObjectGuid::create_player(1, 80);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.initialize_shared_loot_authority_like_cpp(owned_loot_fixture_like_cpp(7, 0));

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    let authority = gameobject.loot_authority_like_cpp().clone();
    let tombstone_generation = authority.generation_like_cpp();
    let lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    assert!(gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &authority,
        tombstone_generation,
        lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(19, 0)),
        HashMap::new(),
    ));
    let installed_generation = authority.generation_like_cpp();
    assert_eq!(
        authority
            .shared_snapshot_like_cpp()
            .expect("restocked shared pool")
            .loot
            .coins,
        19
    );

    assert!(gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &authority,
        tombstone_generation,
        lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(99, 0)),
        HashMap::new(),
    ));
    assert_eq!(authority.generation_like_cpp(), installed_generation);
    assert_eq!(
        authority
            .shared_snapshot_like_cpp()
            .expect("the first shared install remains authoritative")
            .loot
            .coins,
        19,
        "a concurrent generator for the same lifecycle must not replace the first pool"
    );
}

#[test]
fn stale_shared_generator_cannot_cross_second_clear_of_retired_gameobject_lifecycle() {
    let player = ObjectGuid::create_player(1, 81);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.initialize_shared_loot_authority_like_cpp(owned_loot_fixture_like_cpp(3, 0));

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    let stale_authority = gameobject.loot_authority_like_cpp().clone();
    let stale_generation = stale_authority.generation_like_cpp();
    let stale_lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    // `OwnedLootAuthority::retire_like_cpp` is intentionally idempotent for
    // an existing tombstone. The GameObject revision must still reject the
    // async generator captured before this second C++ `ClearLoot`.
    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    assert_eq!(stale_authority.generation_like_cpp(), stale_generation);
    assert!(
        gameobject.loot_lifecycle_revision_like_cpp() > stale_lifecycle_revision,
        "the entity-local lifetime advances even when the authority tombstone does not"
    );

    assert!(!gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &stale_authority,
        stale_generation,
        stale_lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(99, 0)),
        HashMap::new(),
    ));
    assert!(gameobject.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .is_none()
    );
    assert_eq!(gameobject.shared_loot_like_cpp(), None);
}

#[test]
fn shared_generator_cannot_install_after_gameobject_authority_rebind() {
    let player = ObjectGuid::create_player(1, 82);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.initialize_shared_loot_authority_like_cpp(owned_loot_fixture_like_cpp(5, 0));
    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);

    let stale_authority = gameobject.loot_authority_like_cpp().clone();
    let stale_generation = stale_authority.generation_like_cpp();
    let lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();
    let replacement = OwnedLootAuthority::new();
    assert!(gameobject.rebind_loot_authority_like_cpp(replacement.clone()));

    assert!(!gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &stale_authority,
        stale_generation,
        lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(77, 0)),
        HashMap::new(),
    ));
    assert!(replacement.is_pristine_like_cpp());
    assert!(replacement.shared_snapshot_like_cpp().is_none());
}

#[test]
fn gameobject_fully_looted_reads_active_authority_without_summary_refresh() {
    let authority = OwnedLootAuthority::new();
    authority.replace_like_cpp(Some(owned_loot_fixture_like_cpp(31, 0)), HashMap::new());
    let mut gameobject = GameObject::new();
    gameobject.rebind_loot_authority_like_cpp(authority.clone());
    assert_eq!(
        gameobject.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(31, 0))
    );
    assert!(!gameobject.is_fully_looted_like_cpp());

    authority.replace_like_cpp(Some(owned_loot_fixture_like_cpp(0, 0)), HashMap::new());

    assert_eq!(
        gameobject.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(31, 0)),
        "the compatibility summary remains deliberately stale"
    );
    assert!(
        gameobject.is_fully_looted_like_cpp(),
        "lifecycle decisions must read the active object-owned authority"
    );
}

#[test]
fn gameobject_is_fully_looted_checks_shared_and_personal_loot_like_cpp() {
    let mut go = GameObject::new();
    assert!(go.is_fully_looted_like_cpp());
    assert_eq!(go.shared_loot_like_cpp(), None);
    assert_eq!(go.personal_loot_count_like_cpp(), 0);

    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(10, 0));
    assert!(!go.is_fully_looted_like_cpp());

    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::default());
    assert!(go.is_fully_looted_like_cpp());

    let looted_player = ObjectGuid::new(1, 100);
    let unlooted_player = ObjectGuid::new(1, 200);
    go.set_personal_loot_like_cpp(looted_player, GameObjectOwnedLoot::default());
    assert!(go.is_fully_looted_like_cpp());
    assert_eq!(
        go.personal_loot_like_cpp(looted_player),
        Some(&GameObjectOwnedLoot::default())
    );

    go.set_personal_loot_like_cpp(unlooted_player, GameObjectOwnedLoot::new(0, 1));
    assert_eq!(go.personal_loot_count_like_cpp(), 2);
    assert!(!go.is_fully_looted_like_cpp());

    go.set_personal_loot_like_cpp(unlooted_player, GameObjectOwnedLoot::default());
    assert!(go.is_fully_looted_like_cpp());
}
