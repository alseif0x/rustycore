//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn gameobject_post_use_spell_records_missing_spell_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);
    session.set_spell_store(std::sync::Arc::new(wow_data::SpellStore::new()));

    assert!(
        !session.apply_represented_gameobject_post_use_spell_like_cpp(
            gameobject_guid,
            player_guid,
            194097,
            wow_entities::GAMEOBJECT_TYPE_MEETINGSTONE,
            61994,
            false,
            RepresentedGameObjectSpellCaster::User,
            player_guid,
        )
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 194097,
                spell_id: 61994,
                go_type: wow_entities::GAMEOBJECT_TYPE_MEETINGSTONE,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: true,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellMissing {
                gameobject_guid,
                player_guid,
                gameobject_entry: 194097,
                spell_id: 61994,
                go_type: wow_entities::GAMEOBJECT_TYPE_MEETINGSTONE,
                spell_lookup_difficulty_id: 0,
            }
        ]
    );
}
#[test]
fn gameobject_use_fishing_node_activates_ready_bobber_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 17);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .owner_guid = Some(player_guid);

    assert!(
        session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::FishingNodeActivated {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::FinishChanneledSpell { player_guid },
        ]
    );
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert_eq!(state.gameobject_flags, wow_entities::GO_FLAG_IN_MULTI_USE);
}
#[test]
fn gameobject_use_fishing_node_records_skill_roll_and_loot_type_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 47);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.owner_guid = Some(player_guid);
        state.fishing_area_level = Some(200);
        state.player_fishing_level = Some(100);
        state.fishing_roll = Some(30);
    }

    assert!(
        session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingSkillUpdated {
            gameobject_guid,
            player_guid,
        },
    ));
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRoll {
            gameobject_guid,
            player_guid,
            player_fishing_level: 100,
            area_fishing_level: 200,
            chance: 25,
            roll: 30,
        },
    ));
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRequested {
            gameobject_guid,
            player_guid,
            loot_type: wow_packet::packets::loot::LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
        },
    ));
}
#[test]
fn gameobject_use_fishing_node_resolves_area_and_skill_from_session_state_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 57);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 900,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 901,
            continent_id: 0,
            parent_area_id: 900,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_fishing_base_skill_store(Arc::new(
        wow_data::FishingBaseSkillStoreLikeCpp::from_entries([(900, 200)]),
    ));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([
        wow_data::SkillLineEntry {
            id: u32::from(SKILL_FISHING_LIKE_CPP),
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: 9,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        },
        wow_data::SkillLineEntry {
            id: 1_000,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: 9,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: u32::from(SKILL_FISHING_LIKE_CPP),
            parent_tier_index: 4,
            flags: 0,
            spell_book_spell_id: 0,
        },
    ])));
    session.set_player_skill_values_like_cpp(HashMap::from([(1_000, 100)]));
    session.record_represented_gameobject_zone_area_like_cpp(gameobject_guid, 1, 901);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.owner_guid = Some(player_guid);
        state.fishing_roll = Some(30);
    }

    assert!(session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid));

    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRoll {
            gameobject_guid,
            player_guid,
            player_fishing_level: 100,
            area_fishing_level: 200,
            chance: 25,
            roll: 30,
        },
    ));
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRequested {
            gameobject_guid,
            player_guid,
            loot_type: wow_packet::packets::loot::LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
        },
    ));
}
#[test]
fn gameobject_use_fishing_node_resolves_profession_child_skill_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 63);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 900,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_fishing_base_skill_store(Arc::new(
        wow_data::FishingBaseSkillStoreLikeCpp::from_entries([(900, 200)]),
    ));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([
        wow_data::SkillLineEntry {
            id: u32::from(SKILL_FISHING_LIKE_CPP),
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: 9,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        },
        wow_data::SkillLineEntry {
            id: 1_000,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id: 9,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: u32::from(SKILL_FISHING_LIKE_CPP),
            parent_tier_index: 4,
            flags: 0,
            spell_book_spell_id: 0,
        },
    ])));
    session.set_player_skill_values_like_cpp(HashMap::from([
        (SKILL_FISHING_LIKE_CPP, 1),
        (1_000, 100),
    ]));
    session.record_represented_gameobject_zone_area_like_cpp(gameobject_guid, 1, 900);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.owner_guid = Some(player_guid);
        state.fishing_roll = Some(30);
    }

    assert!(session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid));

    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRoll {
            gameobject_guid,
            player_guid,
            player_fishing_level: 100,
            area_fishing_level: 200,
            chance: 25,
            roll: 30,
        },
    ));
}
#[test]
fn gameobject_use_fishing_node_success_clears_canonical_spell_id_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 66);
    let canonical = shared_canonical_map_manager();

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );
    session.record_represented_gameobject_spell_id_like_cpp(gameobject_guid, 3456);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.fishing_area_level = Some(100);
        state.player_fishing_level = Some(100);
        state.fishing_roll = Some(100);
    }

    assert!(session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid));

    let guard = canonical.lock().unwrap();
    let game_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_game_object(gameobject_guid)
        .expect("gameobject inserted into canonical map");
    assert_eq!(game_object.owner_guid(), player_guid);
    assert_eq!(game_object.spell_id(), 0);
}
#[test]
fn gameobject_use_fishing_node_junk_keeps_canonical_spell_id_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 67);
    let canonical = shared_canonical_map_manager();

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );
    session.record_represented_gameobject_spell_id_like_cpp(gameobject_guid, 3456);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.fishing_area_level = Some(500);
        state.player_fishing_level = Some(1);
        state.fishing_roll = Some(100);
    }

    assert!(session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid));

    let guard = canonical.lock().unwrap();
    let game_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_game_object(gameobject_guid)
        .expect("gameobject inserted into canonical map");
    assert!(game_object.owner_guid().is_empty());
    assert_eq!(game_object.spell_id(), 3456);
}
#[test]
fn gameobject_use_fishing_node_wrong_owner_keeps_canonical_spell_id_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let owner_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 68);
    let canonical = shared_canonical_map_manager();

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );
    session.record_represented_gameobject_spell_id_like_cpp(gameobject_guid, 3456);

    assert!(
        !session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid)
    );

    let guard = canonical.lock().unwrap();
    let game_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_game_object(gameobject_guid)
        .expect("gameobject inserted into canonical map");
    assert_eq!(game_object.owner_guid(), owner_guid);
    assert_eq!(game_object.spell_id(), 3456);
}
#[test]
fn gameobject_use_fishing_node_delegates_to_known_fishing_hole_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 48);
    let fishing_hole_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 49);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.owner_guid = Some(player_guid);
        state.fishing_area_level = Some(500);
        state.player_fishing_level = Some(1);
        state.fishing_roll = Some(100);
        state.nearby_fishing_hole_guid = Some(fishing_hole_guid);
    }

    assert!(
        session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingHoleDelegated {
            gameobject_guid,
            player_guid,
            fishing_hole_guid,
        },
    ));
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.loot_state),
        Some(wow_entities::LootState::JustDeactivated)
    );
}
#[test]
fn gameobject_use_fishing_node_finds_nearest_represented_fishing_hole_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 58);
    let farther_hole_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 778, 59);
    let nearest_hole_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 779, 60);

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        farther_hole_guid,
        farther_hole_guid.entry(),
        Position::new(12.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_radius_like_cpp(farther_hole_guid, 20);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        nearest_hole_guid,
        nearest_hole_guid.entry(),
        Position::new(8.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_radius_like_cpp(nearest_hole_guid, 20);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.owner_guid = Some(player_guid);
        state.fishing_area_level = Some(500);
        state.player_fishing_level = Some(1);
        state.fishing_roll = Some(100);
    }

    assert!(
        session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingHoleDelegated {
            gameobject_guid,
            player_guid,
            fishing_hole_guid: nearest_hole_guid,
        },
    ));
    assert!(!session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRequested {
            gameobject_guid,
            player_guid,
            loot_type: wow_packet::packets::loot::LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
        },
    ));
}
#[test]
fn gameobject_use_fishing_node_requires_fishing_hole_radius_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 61);
    let fishing_hole_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 778, 62);

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        gameobject_guid.entry(),
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        fishing_hole_guid,
        fishing_hole_guid.entry(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_radius_like_cpp(fishing_hole_guid, 5);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.owner_guid = Some(player_guid);
        state.fishing_area_level = Some(500);
        state.player_fishing_level = Some(1);
        state.fishing_roll = Some(100);
    }

    assert!(
        session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert!(
        !session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| matches!(
                effect,
                RepresentedGameObjectUseEffect::FishingHoleDelegated { .. }
            ))
    );
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::FishingLootRequested {
            gameobject_guid,
            player_guid,
            loot_type: wow_packet::packets::loot::LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
        },
    ));
}
#[test]
fn gameobject_use_fishing_node_rejects_known_wrong_owner_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let owner_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 17);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .owner_guid = Some(owner_guid);

    assert!(
        !session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::FishingNodeOwnerRejected {
            gameobject_guid,
            player_guid,
            owner_guid,
        }]
    );
}
#[test]
fn gameobject_use_fishing_node_not_ready_sends_not_hooked_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 17);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .loot_state = Some(wow_entities::LootState::Activated);

    assert!(
        session.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::FishNotHooked {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::FinishChanneledSpell { player_guid },
        ]
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.loot_state),
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        (ServerOpcodes::FishNotHooked as u16).to_le_bytes()
    );
}
#[test]
fn gameobject_use_questgiver_sends_gossip_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);

    assert!(session.use_represented_gameobject_questgiver_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::QuestgiverUseSource { gossip_id: 123 },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::SendGossip {
            gameobject_guid,
            player_guid,
            gossip_id: 123,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn gameobject_use_questgiver_single_gameobject_starter_auto_opens_quest_details_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
    let mut quest = test_quest_template(9_001);
    quest.quest_type = 2;
    quest.log_title = "GO starter".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(777, 9_001));
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.use_represented_gameobject_questgiver_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::QuestgiverUseSource { gossip_id: 123 },
    ));

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::SendGossip {
            gameobject_guid,
            player_guid,
            gossip_id: 123,
        }]
    );
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_001]));
}
#[test]
fn gameobject_use_questgiver_ender_relation_precedes_starter_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
    let mut ender = test_quest_template(9_001);
    ender.log_title = "GO ender".into();
    let mut starter = test_quest_template(9_002);
    starter.quest_type = 2;
    starter.log_title = "GO starter".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([ender, starter]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(777, 9_001));
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(777, 9_002));
    session.quests.store = Some(Arc::new(quest_store));
    session.player_quests.insert(
        9_001,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_001,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    assert!(session.use_represented_gameobject_questgiver_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::QuestgiverUseSource { gossip_id: 123 },
    ));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestListMessage)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_001, 9_002]));
}
#[test]
fn quest_giver_query_gameobject_inactive_ender_falls_through_to_same_starter_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 778, 20);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 778, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        778,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest = test_quest_template(9_004);
    quest.quest_type = 2;
    quest.log_title = "GO same source starter".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(778, 9_004));
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(778, 9_004));
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_004));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_004]));
}
#[test]
fn gameobject_use_questgiver_single_incomplete_ender_auto_opens_request_items_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
    let mut ender = test_quest_template(9_003);
    ender.log_title = "GO incomplete ender".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([ender]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(777, 9_003));
    session.quests.store = Some(Arc::new(quest_store));
    session.player_quests.insert(
        9_003,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_003,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    assert!(session.use_represented_gameobject_questgiver_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::QuestgiverUseSource { gossip_id: 123 },
    ));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverRequestItems)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_003]));
    assert_eq!(
        quest_giver_request_items_summary_like_cpp(&bytes),
        QuestGiverRequestItemsSummaryLikeCpp {
            giver_creature_id: 0,
            quest_id: 9_003,
            status_flags: 0xFF,
            auto_launched: true,
        }
    );
}
#[test]
fn gameobject_use_questgiver_without_gameobject_relations_keeps_only_gossip_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_001)]);
    quest_store.starter_quests.insert(777, vec![9_001]);
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.use_represented_gameobject_questgiver_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::QuestgiverUseSource { gossip_id: 123 },
    ));

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::SendGossip {
            gameobject_guid,
            player_guid,
            gossip_id: 123,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn creature_questgiver_single_starter_auto_opens_quest_details_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 21);
    let mut quest = test_quest_template(9_101);
    quest.quest_type = 2;
    quest.log_title = "Creature starter".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    quest_store.starter_quests.insert(777, vec![9_101]);
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.use_represented_creature_questgiver_like_cpp(creature_guid, 777));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_101]));
}
