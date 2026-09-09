//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_only_npc_interaction_preserves_dead_and_ghost_exceptions_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 45);
    let questgiver_guid = test_creature_guid(16);
    let manager = shared_map_manager();

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            questgiver_guid,
            506,
            Position::new(12.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
    );
    session.set_map_manager(Arc::clone(&manager));

    session.set_player_alive_like_cpp(false);
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None
    );

    manager
        .write()
        .unwrap()
        .find_creature_mut(571, 0, questgiver_guid)
        .unwrap()
        .creature
        .set_type_flags_runtime_like_cpp(CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits());
    assert_eq!(
        session
            .represented_npc_can_interact_with_like_cpp(
                questgiver_guid,
                wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
                0,
            )
            .map(|access| access.entry),
        Some(506),
        "C++ allows dead players to interact with creatures visible to ghosts"
    );

    session.set_player_alive_like_cpp(true);
    {
        let mut manager = manager.write().unwrap();
        let creature = manager.find_creature_mut(571, 0, questgiver_guid).unwrap();
        creature
            .creature
            .set_type_flags_runtime_like_cpp(CreatureTypeFlags::empty().bits());
        creature
            .creature
            .set_death_state_runtime(wow_constants::DeathState::JustDied, 0);
        creature.creature.unit_mut().set_health(0);
    }
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None
    );

    manager
        .write()
        .unwrap()
        .find_creature_mut(571, 0, questgiver_guid)
        .unwrap()
        .creature
        .set_type_flags_runtime_like_cpp(CreatureTypeFlags::INTERACT_WHILE_DEAD.bits());
    assert_eq!(
        session
            .represented_npc_can_interact_with_like_cpp(
                questgiver_guid,
                wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
                0,
            )
            .map(|access| access.entry),
        Some(506),
        "C++ allows interaction with dead creatures flagged INTERACT_WHILE_DEAD"
    );
}
#[test]
fn legacy_only_hostile_npc_interaction_is_rejected_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 43);
    let questgiver_guid = test_creature_guid(14);
    let manager = shared_map_manager();

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_faction_template_like_cpp(1);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(35, 35, 0, 0, 1),
            faction_template_entry(1, 1, 0, 0, 0),
        ]),
    ));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            questgiver_guid,
            504,
            Position::new(14.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
    );
    session.set_map_manager(Arc::clone(&manager));

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None
    );

    manager
        .write()
        .unwrap()
        .find_creature_mut(571, 0, questgiver_guid)
        .unwrap()
        .creature
        .set_unit_flags2_runtime_like_cpp(UnitFlags2::INTERACT_WHILE_HOSTILE.bits());

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 504,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
}
#[test]
fn legacy_only_charmed_npc_interaction_is_rejected_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 47);
    let questgiver_guid = test_creature_guid(18);
    let manager = shared_map_manager();

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let mut creature = crate::map_manager::WorldCreature::new(
        questgiver_guid,
        508,
        Position::new(14.0, 0.0, 0.0, 0.0),
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
        0,
    );
    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_charmer(player_guid, true);
    manager
        .write()
        .unwrap()
        .add_creature(571, 0, 0, 0, creature);
    session.set_map_manager(Arc::clone(&manager));

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None,
        "C++ Player::GetNPCIfCanInteractWith rejects creatures with GetCharmerGUID set"
    );
}
#[test]
fn canonical_player_same_target_sync_preserves_mutable_state_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 53);
    let target_guid = test_creature_guid(19_053);
    let updated_position = Position::new(3800.0, 1525.0, 120.0, 2.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SameTarget".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(target_guid));
            player.set_player_flag(PLAYER_FLAGS_UBER_LIKE_CPP);
        })
        .unwrap();

    assert!(session.ensure_canonical_player_owner_for_map_like_cpp(
        wow_map::MapKey::new(571, 0),
        updated_position,
    ));

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player_guid)
        .unwrap();
    assert_eq!(player.unit().attacking(), Some(target_guid));
    assert!(player.has_player_flag(PLAYER_FLAGS_UBER_LIKE_CPP));
    assert_eq!(player.unit().world().position(), updated_position);
    let cell = wow_map::cell_from_world(updated_position.x, updated_position.y);
    assert_eq!(
        player.unit().world().current_cell(),
        Some((cell.cell_x(), cell.cell_y())),
        "same-map residence must preserve C++ Map::PlayerRelocation cell membership"
    );
}
#[test]
fn canonical_player_vitals_follow_active_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_557);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VitalOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(321);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(123);
        })
        .expect("active canonical Player");

    assert_eq!(
        session.resolved_player_vitals_like_cpp(),
        Some((123, 321, true))
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_player_vitals_like_cpp(),
        Some((123, 321, true))
    );

    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.unit_mut().set_max_health(999);
    replacement.unit_mut().set_health(999);
    canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_vitals_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        None
    );
}
#[test]
fn canonical_player_powers_follow_active_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_558);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PowerOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    session.set_loaded_player_powers_like_cpp([123, 45, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(session.sync_canonical_player_primary_power_like_cpp(PowerType::Mana, 123, 321, 222,));

    assert_eq!(
        session.represented_player_power_values_like_cpp(),
        Some([123, 45, 0, 0, 0, 0, 0, 0, 0, 0])
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.represented_player_power_values_like_cpp(),
        Some([123, 45, 0, 0, 0, 0, 0, 0, 0, 0])
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(old_handle, |player| (
                player.unit().get_max_power(PowerType::Mana),
                player.unit().get_create_mana_like_cpp(),
            )),
        Some((321, 222))
    );

    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.represented_player_power_values_like_cpp(), None);
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        None
    );
}
#[test]
fn canonical_player_progression_follows_active_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_559);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ProgressionOwner".to_string(),
        position,
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    assert!(session.set_player_xp_like_cpp(123));
    assert!(session.set_player_next_level_xp_like_cpp(456));
    assert!(session.set_player_character_points_like_cpp(7));

    assert_eq!(session.resolved_player_xp_like_cpp(), Some(123));
    assert_eq!(session.resolved_player_next_level_xp_like_cpp(), Some(456));
    assert_eq!(session.resolved_player_character_points_like_cpp(), Some(7));
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.resolved_player_xp_like_cpp(), Some(123));
    assert_eq!(session.resolved_player_next_level_xp_like_cpp(), Some(456));
    assert_eq!(session.resolved_player_character_points_like_cpp(), Some(7));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_xp(900);
    replacement.set_next_level_xp(1_000);
    replacement.set_character_points_like_cpp(11);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_xp_like_cpp(), None);
    assert_eq!(session.resolved_player_next_level_xp_like_cpp(), None);
    assert_eq!(session.resolved_player_character_points_like_cpp(), None);
    assert!(!session.set_player_xp_like_cpp(1));
    assert!(!session.set_player_next_level_xp_like_cpp(2));
    assert!(!session.set_player_character_points_like_cpp(3));
    assert!(!session.give_xp_runtime_like_cpp(10, ObjectGuid::EMPTY, 1.0));
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.active_data().xp,
                player.active_data().next_level_xp,
                player.active_data().character_points,
            )),
        Some((900, 1_000, 11))
    );
}
#[test]
fn canonical_player_specialization_metadata_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_560);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SpecializationOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    assert!(session.set_player_create_mode_like_cpp(1));
    assert!(session.set_represented_shapeshift_form_like_cpp(5));
    assert!(session.set_loot_specialization_id_like_cpp(65));
    assert!(session.set_represented_primary_specialization_id_like_cpp(66));

    assert_eq!(session.player_create_mode_like_cpp(), Some(1));
    assert_eq!(session.represented_shapeshift_form_like_cpp(), Some(5));
    assert_eq!(session.loot_specialization_id_like_cpp(), Some(65));
    assert_eq!(
        session.represented_primary_specialization_id_like_cpp(),
        Some(66)
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.player_create_mode_like_cpp(), Some(1));
    assert_eq!(session.represented_shapeshift_form_like_cpp(), Some(5));
    assert_eq!(session.loot_specialization_id_like_cpp(), Some(65));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_create_mode_like_cpp(0);
    replacement.set_shapeshift_form_id_like_cpp(2);
    replacement.set_loot_specialization_id_like_cpp(70);
    replacement.set_primary_specialization(71);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_create_mode_like_cpp(), None);
    assert_eq!(session.represented_shapeshift_form_like_cpp(), None);
    assert_eq!(session.loot_specialization_id_like_cpp(), None);
    assert_eq!(
        session.represented_primary_specialization_id_like_cpp(),
        None
    );
    assert!(!session.set_player_create_mode_like_cpp(1));
    assert!(!session.set_represented_shapeshift_form_like_cpp(5));
    assert!(!session.set_loot_specialization_id_like_cpp(65));
    assert!(!session.set_represented_primary_specialization_id_like_cpp(66));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.create_mode_like_cpp(),
                player.shapeshift_form_id_like_cpp(),
                player.loot_specialization_id_like_cpp(),
                player.primary_specialization_id_like_cpp(),
            )),
        Some((0, 2, 70, 71))
    );
}
#[test]
fn canonical_player_battleground_context_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_570);
    let queue_type = RepresentedBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: 3,
        queue_type: 1,
        rated: false,
        team_size: 0,
    };

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "BattlegroundOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_player_battleground_context_like_cpp(3, 529));
    assert!(session.set_represented_arena_team_id_invited_like_cpp(77));
    session.set_represented_battleground_status_like_cpp(Some(4));
    session.add_represented_battleground_queue_slot_like_cpp(1, queue_type, 88);
    assert!(session.player_in_represented_battleground_like_cpp());
    assert!(session.represented_battleground_status_is_wait_leave_like_cpp());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(session.player_in_represented_battleground_like_cpp());
    assert!(session.represented_battleground_status_is_wait_leave_like_cpp());
    assert_eq!(session.represented_arena_team_id_invited_like_cpp(), 77);

    let replacement_state = wow_entities::PlayerBattlegroundState {
        represented_type_id: Some(7),
        represented_map_id: Some(30),
        represented_status: Some(3),
        represented_queue_slots: vec![wow_entities::PlayerBattlegroundQueueSlotLikeCpp {
            slot: 2,
            queue_type_id: queue_type,
            invited_instance_guid: 99,
        }],
        arena_team_id_invited: 100,
        ..Default::default()
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().battleground = replacement_state.clone();
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(!session.player_in_represented_battleground_like_cpp());
    assert!(!session.set_player_battleground_context_like_cpp(1, 489));
    assert!(!session.set_represented_arena_team_id_invited_like_cpp(101));
    session.set_represented_battleground_status_like_cpp(Some(4));
    assert!(!session.represented_battleground_status_is_wait_leave_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().battleground.clone()
            }),
        Some(replacement_state)
    );
}
#[test]
fn canonical_player_menu_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_574);
    let source_guid = test_creature_guid(574);
    let option = GossipOptionInfo {
        gossip_option_id: 7,
        menu_id: 11,
        order_index: 2,
        option_npc: 3,
        action_menu_id: 13,
    };

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MenuOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_player_trainer_interaction_like_cpp(source_guid, 77));
    assert!(session.replace_player_gossip_options_like_cpp(vec![option.clone()]));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(source_guid)
    );
    assert!(session.player_trainer_interaction_matches_like_cpp(source_guid, 77));
    assert_eq!(
        session.player_gossip_option_like_cpp(7),
        Some(option.clone())
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.player_gossip_option_like_cpp(7), Some(option));

    let replacement_source = test_creature_guid(575);
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_trainer_interaction_like_cpp(replacement_source, 99);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_interaction_source_guid_like_cpp(), None);
    assert_eq!(
        session.resolved_player_interaction_trainer_id_like_cpp(),
        None
    );
    assert_eq!(session.player_gossip_option_like_cpp(7), None);
    assert!(!session.set_player_interaction_source_like_cpp(source_guid));
    assert!(!session.replace_player_gossip_options_like_cpp(Vec::new()));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                *player.interaction_data_like_cpp()
            }),
        Some(wow_entities::PlayerInteractionDataLikeCpp {
            source_guid: replacement_source,
            trainer_id: 99,
            player_choice_id: 0,
        })
    );
}
#[test]
fn canonical_player_outdoors_state_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_580);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "OutdoorsOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    session.set_represented_is_outdoors_like_cpp(true);
    assert_eq!(
        session
            .player_world_local_state_like_cpp()
            .and_then(|state| state.is_outdoors),
        Some(true)
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    session.set_represented_is_outdoors_like_cpp(false);
    assert_eq!(
        session
            .player_world_local_state_like_cpp()
            .and_then(|state| state.is_outdoors),
        Some(false)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().world_local.is_outdoors = Some(true);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_world_local_state_like_cpp(), None);
    session.set_represented_is_outdoors_like_cpp(false);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .gameplay_state()
                .world_local
                .is_outdoors,),
        Some(Some(true))
    );
}
