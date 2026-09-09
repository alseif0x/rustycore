//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_sanctuary_in_dungeon_scales_player_threat_to_zero_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let threat_spell_id = 791_i32;
    let sanctuary_spell_id = 792_i32;
    let player_guid = ObjectGuid::create_player(1, 792);
    let creature_guid = test_creature_guid(18_792);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SanctuaryDungeonTarget".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        threat_spell_id,
        threat_spell_info_like_cpp(
            threat_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
            35,
        ),
    );
    spell_store.insert(
        sanctuary_spell_id,
        threat_spell_info_like_cpp(
            sanctuary_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SANCTUARY,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(threat_spell_id, creature_guid)
        .await
        .expect("represented EffectThreat should execute");
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(35.0)
    );
    session
        .execute_spell(sanctuary_spell_id, player_guid)
        .await
        .expect("represented EffectSanctuary should execute");

    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(0.0)
    );
    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(0.0));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::CooldownEvent,
            ServerOpcodes::SpellGo,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn killing_moving_creature_sends_cpp_like_monster_move_stop() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_202);
    session.player_guid = Some(ObjectGuid::create_player(1, 202));
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .begin_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
                .expect("valid represented spline");
        })
        .unwrap();

    session.apply_damage(None, guid, 40).await.unwrap();

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::OnMonsterMove as u16);
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_float().unwrap(), 10.0);
    assert_eq!(pkt.read_float().unwrap(), 10.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_uint32().unwrap(), 3); // spline id
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    // CrzTeleport + StopDistanceTolerance precede Flags on the wire: the
    // spline's first integer write flushes those 4 bits into their own byte.
    assert!(!pkt.has_bit().unwrap()); // CrzTeleport
    assert_eq!(pkt.read_bits(3).unwrap(), 2); // StopDistanceTolerance
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Flags
    assert_eq!(pkt.read_int32().unwrap(), 0); // Elapsed
    assert_eq!(pkt.read_uint32().unwrap(), 0); // MoveTime
    assert_eq!(pkt.read_uint32().unwrap(), 0); // FadeObjectTime
    assert_eq!(pkt.read_uint8().unwrap(), 0); // Mode
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY); // TransportGUID
    assert_eq!(pkt.read_int8().unwrap(), -1); // VehicleSeat
    assert_eq!(pkt.read_bits(2).unwrap(), 0); // Face
    assert_eq!(pkt.read_bits(16).unwrap(), 0); // Points.len()
    assert!(!pkt.has_bit().unwrap()); // VehicleExitVoluntary
    assert!(!pkt.has_bit().unwrap()); // Interpolate
    assert_eq!(pkt.read_bits(16).unwrap(), 0); // PackedDeltas.len()
    assert!(!pkt.has_bit().unwrap()); // SplineFilter
    assert!(!pkt.has_bit().unwrap()); // SpellEffectExtraData
    assert!(!pkt.has_bit().unwrap()); // JumpExtraData
    assert!(!pkt.has_bit().unwrap()); // AnimTierTransition
}
#[test]
fn combat_tick_damage_syncs_canonical_creature_health() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_003);
    let player = ObjectGuid::create_player(1, 43);
    session.player_guid = Some(player);
    session.combat_target = Some(guid);
    session.in_combat = true;
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.current_hp() < 40);

    let attacker_state = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([attacker_state[0], attacker_state[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackerStateUpdate as u16);
    let values_update = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([values_update[0], values_update[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
}
#[tokio::test]
async fn combat_tick_kill_keeps_empty_creature_loot_non_lootable_after_pending_drain_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_015);
    let player = ObjectGuid::create_player(1, 63);
    session.player_guid = Some(player);
    session.set_player_level_like_cpp(1);
    session.set_player_xp_like_cpp(0);
    session.set_player_next_level_xp_like_cpp(400);
    session.combat_target = Some(guid);
    session.in_combat = true;
    let mut quest_store = wow_data::quest::QuestStore::new();
    quest_store.quests.insert(
        9_001,
        wow_data::quest::QuestTemplate {
            id: 9_001,
            quest_type: 0,
            quest_level: 1,
            quest_max_scaling_level: 0,
            quest_package_id: 0,
            min_level: 1,
            quest_sort_id: 0,
            quest_info_id: 0,
            suggested_group_num: 0,
            reward_next_quest: 0,
            reward_xp_difficulty: 0,
            reward_xp_multiplier: 1.0,
            reward_money_difficulty: 0,
            reward_money_multiplier: 1.0,
            reward_bonus_money: 0,
            reward_display_spell: [0; wow_data::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
            reward_spell: 0,
            reward_honor: 0,
            reward_title_id: 0,
            reward_skill_line_id: 0,
            reward_skill_points: 0,
            reward_mail_template_id: 0,
            reward_mail_delay_secs: 0,
            reward_mail_sender_entry: 0,
            reward_faction_ids: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_values: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_overrides: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_cap_in: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_flags: 0,
            source_item_id: 0,
            source_item_count: 0,
            source_spell_id: 0,
            limit_time_secs: 0,
            expansion: 0,
            flags: 0,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags: 0,
            event_id_for_quest: 0,
            reward_items: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
            reward_amounts: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
            reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
            reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
            item_drop: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
            item_drop_quantity: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
            log_title: String::new(),
            log_description: String::new(),
            quest_description: String::new(),
            area_description: String::new(),
            quest_completion_log: String::new(),
            objectives: vec![wow_data::quest::QuestObjective {
                id: 1,
                quest_id: 9_001,
                obj_type: 0,
                order: 0,
                storage_index: 0,
                object_id: 9001,
                amount: 1,
                flags: 0,
                flags2: 0,
                progress_bar_weight: 0.0,
                description: String::new(),
            }],
            allowable_races: 0,
            allowable_classes: 0,
            max_level: 0,
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            breadcrumb_for_quest_id: 0,
            dependent_previous_quests: Vec::new(),
            dependent_breadcrumb_quests: Vec::new(),
            required_min_rep_faction: 0,
            required_min_rep_value: 0,
            required_max_rep_faction: 0,
            required_max_rep_value: 0,
            required_skill_id: 0,
            required_skill_points: 0,
            reward_choice_items: [(0, 0); wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
            reward_choice_item_types: [0; wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
        },
    );
    session.set_quest_store(Arc::new(quest_store));
    session.player_quests.insert(
        9_001,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_001,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );
    register_test_creature(&mut session, manager.clone(), guid, 3);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    assert!(session.loot_table.get(&guid).is_none());
    assert_eq!(session.pending_creature_kill_loot_like_cpp, vec![guid]);

    session.process_pending().await;

    let loot = session
        .loot_table
        .get(&guid)
        .expect("melee kill loot is generated from pending bridge");
    assert!(loot.allowed_looters.contains(&player));
    assert_eq!(loot.loot_type, LOOT_TYPE_CORPSE_LIKE_CPP);
    assert_eq!((loot.coins, loot.unlooted_count), (0, 0));
    assert!(session.player_xp_like_cpp() > 0);
    let quest = session.player_quests.get(&9_001).unwrap();
    assert_eq!(
        quest.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_eq!(quest.objective_counts, vec![1]);
    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.creature.is_tapped_by(player));
    assert!(
        !world_creature
            .creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    );
    assert!(
        !world_creature
            .creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE)
    );
    assert_eq!(
        session.represented_creature_kill_events_like_cpp(),
        &[
            RepresentedCreatureKillEventLikeCpp::KillerProc {
                attacker_guid: player,
                victim_guid: guid,
            },
            RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
                tapper_guid: player,
                victim_guid: guid,
            },
            RepresentedCreatureKillEventLikeCpp::VictimDeathProc { victim_guid: guid },
            RepresentedCreatureKillEventLikeCpp::DeliveredKillingBlowCriteria {
                player_guid: player,
                victim_guid: guid,
                quantity: 1,
            },
            RepresentedCreatureKillEventLikeCpp::DeathStateJustDied { victim_guid: guid },
            RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath { unit_guid: guid },
            RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
                creature_guid: guid,
                lootable: false,
                can_skin: false,
                skinnable: false,
            },
            RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
                creature_guid: guid,
                attacker_guid: player,
                is_kill: true,
            },
            RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi {
                creature_guid: guid,
                killer_guid: player,
            },
            RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
                killer_guid: player,
                creature_guid: guid,
            },
        ]
    );
}
#[test]
fn combat_tick_damage_adds_creature_threat_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_024);
    let player = ObjectGuid::create_player(1, 73);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Threat".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    assert!(session.mirror_canonical_creature_threat_from_attacker_like_cpp(guid, player, 3.0));

    session.tick_combat_sync();

    let guard = manager.read().unwrap();
    let world_creature = guard.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.creature.is_tapped_by(player));
    assert!(world_creature.creature.has_loot_recipient());
    assert_eq!(
        world_creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(player),
        Some(7.0)
    );
    drop(guard);

    let canonical = canonical.lock().unwrap();
    let player_entity = canonical
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity.unit().last_damaged_target_like_cpp(),
        Some(guid)
    );
    assert!(
        player_entity
            .unit()
            .subsystems()
            .combat
            .is_threatening_to(guid, true)
    );
    assert!(
        player_entity
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(guid)
    );
    let creature_entity = canonical
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap();
    assert_eq!(
        creature_entity
            .unit()
            .subsystems()
            .combat
            .threat_value(player),
        Some(10.0)
    );
    assert!(
        creature_entity
            .unit()
            .subsystems()
            .combat
            .is_in_combat_with(player)
    );
}
#[tokio::test]
async fn attack_stop_preserves_creature_combat_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_004);
    let player = ObjectGuid::create_player(1, 44);
    session.player_guid = Some(player);
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| creature.enter_combat(player))
        .unwrap();

    session
        .handle_attack_stop(WorldPacket::from_bytes(&[]))
        .await;

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(
        world_creature.creature.ai_state(),
        wow_entities::CreatureAiState::InCombat
    );
    assert_eq!(
        world_creature.creature.ai_ownership().combat_target,
        Some(player)
    );
}
#[test]
fn player_attack_tracks_world_creature_attacker_set_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 75);
    let victim = test_creature_guid(18_026);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    register_test_creature(&mut session, manager.clone(), victim, 40);

    session.start_player_attack_like_cpp(victim);
    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, victim).unwrap();
        assert!(creature.creature.unit().has_attacker_like_cpp(attacker));
    }

    assert_eq!(session.stop_player_attack_like_cpp(), Some(victim));
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, victim).unwrap();
    assert!(!creature.creature.unit().has_attacker_like_cpp(attacker));
}
#[test]
fn give_xp_runtime_rejects_creature_without_loot_recipient_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let victim = test_creature_guid(0xE1C1);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    install_xp_victim_like_cpp(&mut session, victim, false);

    assert!(!session.give_xp_runtime_like_cpp(50, victim, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 70.0);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());

    let player_guid = session.player_guid().expect("test player");
    session
        .mutate_world_creature(victim, |creature| {
            creature.creature.set_tapped_by_player(player_guid, &[]);
        })
        .expect("test victim");
    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 100);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 20.0);
}
#[test]
fn player_attack_tracks_typed_creature_victim_attacker_set_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 50);
    let victim = test_creature_guid(18_008);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(victim, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );

    session.start_player_attack_like_cpp(victim);
    {
        let guard = canonical.lock().unwrap();
        let creature = guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(victim, Clone::clone)
            .unwrap();
        assert!(creature.unit().has_attacker_like_cpp(player));
        assert!(
            creature
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(player)
        );
        let player_entity = guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap();
        assert!(
            player_entity
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(victim)
        );
    }

    assert_eq!(session.stop_player_attack_like_cpp(), Some(victim));
    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap();
    assert!(!creature.unit().has_attacker_like_cpp(player));
}
#[test]
fn player_attack_dead_typed_creature_is_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 51);
    let victim = test_creature_guid(18_009);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(victim, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_world_creature(victim, |creature| {
            creature.take_damage(25);
        })
        .unwrap();

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        None
    );
    assert!(
        !map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(player)
    );
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
#[test]
fn player_attack_unseen_phase_creature_is_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 151);
    let victim = test_creature_guid(18_109);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    let mut player_phase = PhaseShift::default();
    player_phase.add_phase_like_cpp(10, wow_constants::PhaseFlags::empty(), 1);
    session.set_represented_player_phase_shift_like_cpp(player_phase);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(victim, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_canonical_creature_by_guid_like_cpp(victim, |creature| {
            let mut victim_phase = PhaseShift::default();
            victim_phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);
            *creature.unit_mut().world_mut().phase_shift_mut() = victim_phase;
        })
        .unwrap();

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    let player_entity = map.get_typed_player(player).unwrap();
    let victim_entity = map.with_creature_like_cpp(victim, Clone::clone).unwrap();
    assert_eq!(player_entity.unit().attacking(), None);
    assert_eq!(player_entity.unit().data().target, ObjectGuid::EMPTY);
    assert!(!victim_entity.unit().has_attacker_like_cpp(player));
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
