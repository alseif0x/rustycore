//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_gameobject_runtime_state_upserts_into_player_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_710);
    let gameobject_guid = test_gameobject_guid(61_711, 61_711);
    let position = Position::new(40.0, 50.0, 60.0, 2.0);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        9_301,
        position,
        wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
    );

    let guard = canonical.lock().unwrap();
    assert!(
        guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_game_object(gameobject_guid)
            .is_none()
    );
    let gameobject = guard
        .find_map(571, 7)
        .unwrap()
        .map()
        .get_typed_game_object(gameobject_guid)
        .expect("represented client-visible GO is materialized in the player's instance");
    assert_eq!(gameobject.world().map_id(), 571);
    assert_eq!(gameobject.world().instance_id(), 7);
    assert_eq!(gameobject.world().position(), position);
}
#[test]
fn represented_gameobject_runtime_state_uses_typed_canonical_map_object_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 9001, 44);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        guid,
        9001,
        Position::new(10.0, 20.0, 30.0, 1.0),
        wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
    );

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    assert_eq!(map.get_game_object(guid).unwrap().guid(), guid);
    let typed = map
        .get_typed_game_object(guid)
        .expect("gameobject stored as typed GameObject entity");
    assert_eq!(typed.world().object().entry(), 9001);
    assert_eq!(
        typed.world().position(),
        Position::new(10.0, 20.0, 30.0, 1.0)
    );
}
#[test]
fn represented_gameobject_runtime_state_captures_canonical_linked_trap_guid_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 9002, 45);
    let trap_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 9003, 46);
    let position = Position::new(10.0, 20.0, 30.0, 1.0);

    add_canonical_test_gameobject(&canonical, guid, 9002, position);
    add_canonical_test_gameobject(&canonical, trap_guid, 9003, position);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_game_object_mut(guid)
        .unwrap()
        .set_linked_trap_like_cpp(trap_guid);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        guid,
        9002,
        position,
        wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
    );

    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.linked_trap_guid),
        Some(trap_guid)
    );
}
#[test]
fn remove_world_creature_removes_canonical_map_object_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(613);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.register_world_creature(
        571,
        Position::new(10.0, 20.0, 30.0, 1.0),
        test_creature_create_data(guid, 9001, 25),
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

    assert!(session.remove_world_creature(guid).is_some());
    let guard = canonical.lock().unwrap();
    assert!(
        guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_creature(guid)
            .is_none()
    );
}
#[test]
fn register_world_creature_applies_valid_terrain_swap_visible_map_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(609);
    let map_store = Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]));
    let terrain_swap_store = Arc::new(wow_data::TerrainSwapStore::from_rows_like_cpp(
        &map_store,
        [],
        [],
        |_| true,
    ));

    session.set_map_manager(Arc::clone(&manager));
    session.set_map_store(map_store);
    session.set_terrain_swap_store(terrain_swap_store);
    session.register_world_creature(
        571,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9001, 25),
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
        609,
    );

    let guard = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let creature = guard.find_creature(571, 0, guid).unwrap();
    assert!(creature.phase_shift().has_visible_map_id_like_cpp(609));
    assert_eq!(creature.creature.ai_ownership().terrain_swap_map, 609);
}
#[test]
fn register_world_creature_applies_db_phase_shift_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(710);
    let phase_store = Arc::new(wow_data::PhaseStore::from_entries([
        wow_data::PhaseEntry { id: 10, flags: 0 },
        wow_data::PhaseEntry { id: 20, flags: 0 },
    ]));
    let phase_group_store = Arc::new(wow_data::PhaseGroupStore::from_entries(
        &phase_store,
        [wow_data::PhaseXPhaseGroupEntry {
            id: 1,
            phase_id: 20,
            phase_group_id: 7,
        }],
    ));

    session.set_map_manager(Arc::clone(&manager));
    session.set_phase_store(phase_store);
    session.set_phase_group_store(phase_group_store);
    session.register_world_creature(
        571,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        crate::phasing::PHASE_USE_FLAGS_INVERSE,
        0,
        7,
        -1,
    );

    let guard = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let creature = guard.find_creature(571, 0, guid).unwrap();
    assert!(creature.phase_shift().is_db_phase_shift_like_cpp());
    assert!(creature.phase_shift().has_phase_like_cpp(20));
    assert_eq!(
        creature.phase_shift().flags_like_cpp(),
        PhaseShiftFlags::INVERSE
    );
    assert_eq!(
        creature.creature.ai_ownership().phase_use_flags,
        crate::phasing::PHASE_USE_FLAGS_INVERSE
    );
    assert_eq!(creature.creature.ai_ownership().phase_group_id, 7);
}
#[test]
fn represented_gameobject_phase_shift_applies_db_phase_and_visible_map_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 1, 900);
    let phase_store = Arc::new(wow_data::PhaseStore::from_entries([wow_data::PhaseEntry {
        id: 20,
        flags: 0,
    }]));
    let phase_group_store = Arc::new(wow_data::PhaseGroupStore::from_entries(
        &phase_store,
        [wow_data::PhaseXPhaseGroupEntry {
            id: 1,
            phase_id: 20,
            phase_group_id: 7,
        }],
    ));
    let map_store = Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]));
    let terrain_swap_store = Arc::new(wow_data::TerrainSwapStore::from_rows_like_cpp(
        &map_store,
        [],
        [],
        |_| true,
    ));

    session.set_phase_store(phase_store);
    session.set_phase_group_store(phase_group_store);
    session.set_map_store(map_store);
    session.set_terrain_swap_store(terrain_swap_store);
    session.record_represented_gameobject_db_phase_shift_like_cpp(
        guid,
        571,
        crate::phasing::PHASE_USE_FLAGS_INVERSE,
        0,
        7,
        609,
    );

    let phase_shift = session
        .represented_gameobject_phase_shifts
        .get(&guid)
        .unwrap();
    assert!(phase_shift.is_db_phase_shift_like_cpp());
    assert!(phase_shift.has_phase_like_cpp(20));
    assert!(phase_shift.has_visible_map_id_like_cpp(609));
    assert_eq!(phase_shift.flags_like_cpp(), PhaseShiftFlags::INVERSE);
}
#[test]
fn session_db_spawn_phase_visibility_uses_player_phase_can_see_like_cpp() {
    let (mut session, _, _) = make_session();
    let phase_store = Arc::new(wow_data::PhaseStore::from_entries([wow_data::PhaseEntry {
        id: 20,
        flags: 0,
    }]));
    let phase_group_store = Arc::new(wow_data::PhaseGroupStore::from_entries(
        &phase_store,
        [wow_data::PhaseXPhaseGroupEntry {
            id: 1,
            phase_id: 20,
            phase_group_id: 7,
        }],
    ));

    session.set_phase_store(Arc::clone(&phase_store));
    session.set_phase_group_store(Arc::clone(&phase_group_store));

    let (target_phase_shift, _) = session.db_spawn_phase_shift_like_cpp(571, 0, 20, 0, -1);
    assert!(!session.can_see_phase_shift_like_cpp(&target_phase_shift));

    let mut player_phase_shift = PhaseShift::default();
    init_db_phase_shift_like_cpp(
        &mut player_phase_shift,
        &phase_store,
        &phase_group_store,
        0,
        20,
        0,
    );
    session.set_represented_player_phase_shift_like_cpp(player_phase_shift);
    assert!(session.can_see_phase_shift_like_cpp(&target_phase_shift));

    let (always_visible_shift, _) = session.db_spawn_phase_shift_like_cpp(
        571,
        crate::phasing::PHASE_USE_FLAGS_ALWAYS_VISIBLE,
        20,
        0,
        -1,
    );
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::default());
    assert!(session.can_see_phase_shift_like_cpp(&always_visible_shift));
}
#[test]
fn tick_creatures_sync_launches_real_move_spline_for_represented_wander() {
    let (mut session, _, send_rx) = make_session();
    session.set_mmap_runtime_config_like_cpp(MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    });
    let manager = shared_map_manager();
    let guid = test_creature_guid(77);
    register_test_creature(&mut session, manager, guid, 25);
    session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x5757);
        })
        .unwrap();
    session.client_visible_guids_like_cpp.insert(guid);

    let sent = (0..32)
        .find_map(|_| {
            session.tick_creatures_sync();
            send_rx.try_recv().ok()
        })
        .expect("random movement should eventually launch a MonsterMove packet");
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::OnMonsterMove as u16);
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_float().unwrap(), 10.0);
    assert_eq!(pkt.read_float().unwrap(), 10.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_uint32().unwrap(), 2);
    let packet_destination = Position::new(
        pkt.read_float().unwrap(),
        pkt.read_float().unwrap(),
        pkt.read_float().unwrap(),
        0.0,
    );
    assert_eq!(packet_destination, Position::ZERO);
    assert!(!pkt.is_empty());
    assert!(send_rx.try_recv().is_err());

    session
        .mutate_world_creature(guid, |creature| {
            let motion_spline = &creature.creature.unit().subsystems().motion.spline;
            assert!(motion_spline.enabled);
            assert!(!motion_spline.finalized);
            assert_eq!(motion_spline.spline_id, 2);
            assert!(motion_spline.duration_ms > 0);
            assert_eq!(motion_spline.final_destination, Some((10, 7, 0)));
            assert_eq!(
                creature.state(),
                wow_entities::CreatureAiState::WalkingRandom
            );
        })
        .unwrap();
}
#[tokio::test]
async fn spell_damage_syncs_canonical_creature_health() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_002);
    let player = ObjectGuid::create_player(1, 42);
    session.player_guid = Some(player);
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);

    session.apply_damage(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 33);
    assert!(world_creature.creature.is_tapped_by(player));
    assert!(world_creature.creature.has_loot_recipient());
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
}
#[tokio::test]
async fn spell_damage_effects_add_pct_threat_and_one_cast_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_003);
    let player = ObjectGuid::create_player(1, 43);
    let spell_id = 18_003;
    session.player_guid = Some(player);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let mut spell_store = wow_data::SpellStore::new();
    let mut damage_spell = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
        7,
    );
    damage_spell.effects.push(wow_data::SpellEffectInfo {
        effect_index: 1,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
        effect_base_points: 7,
        ..Default::default()
    });
    spell_store.insert(spell_id, damage_spell);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: HashMap::from([(
            spell_id as u32,
            wow_data::SpellThreatEntryLikeCpp {
                flat_mod: 5,
                pct_mod: 2.0,
                ap_pct_mod: 0.5,
            },
        )]),
    }));
    session
        .represented_item_bonus_state_like_cpp
        .attack_power_total = 10;

    session.execute_spell(spell_id, guid).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(
        world_creature.state(),
        wow_entities::CreatureAiState::InCombat
    );
    assert_eq!(
        world_creature.creature.ai_ownership().combat_target,
        Some(player)
    );
    let combat = &world_creature.creature.unit().subsystems().combat;
    assert_eq!(
        combat.threat_value(player),
        Some(38.0),
        "C++ damage threat applies pctMod, but harmful cast-level threat bypasses modifiers"
    );
    assert_eq!(combat.current_victim_guid, Some(player));
}
#[test]
fn hostile_cast_level_threat_enters_combat_without_damage_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let creature_guid = test_creature_guid(18_004);
    let caster_guid = ObjectGuid::create_player(1, 44);
    let spell_id = 18_004;
    session.player_guid = Some(caster_guid);
    session.state = SessionState::LoggedIn;
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
            0,
        ),
    );
    session.set_spell_store(Arc::new(store));
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: HashMap::from([(
            spell_id as u32,
            wow_data::SpellThreatEntryLikeCpp {
                flat_mod: 7,
                pct_mod: 3.0,
                ap_pct_mod: 0.0,
            },
        )]),
    }));

    session.apply_spell_initial_threat_like_cpp(spell_id, caster_guid, creature_guid, false);

    let manager = manager.read().unwrap();
    let creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert!(creature.creature.is_in_combat());
    assert_eq!(
        creature.creature.ai_ownership().combat_target,
        Some(caster_guid)
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(caster_guid),
        Some(7.0)
    );
    let packet = send_rx.try_recv().expect("spell pull attack-start");
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::AttackStart as u16
    );
}
#[tokio::test]
async fn spell_damage_no_threat_attributes_skip_engage_and_threat_like_cpp() {
    for (counter, attribute_index, attribute) in [
        (
            18_005,
            4,
            wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
        ),
        (
            18_006,
            1,
            wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT,
        ),
    ] {
        let (mut session, _, _) = make_session();
        let manager = shared_map_manager();
        let guid = test_creature_guid(counter);
        let player = ObjectGuid::create_player(1, counter);
        let spell_id = counter as i32;
        session.player_guid = Some(player);
        register_test_creature(&mut session, manager.clone(), guid, 40);
        let mut spell_store = wow_data::SpellStore::new();
        let mut attributes = [0; 15];
        attributes[attribute_index] = attribute;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        session.set_spell_store(Arc::new(spell_store));

        session.apply_damage(Some(spell_id), guid, 7).await.unwrap();

        let manager = manager.read().unwrap();
        let creature = manager.find_creature(0, 0, guid).unwrap();
        assert_eq!(creature.current_hp(), 33);
        assert!(!creature.creature.is_in_combat());
        assert_eq!(
            creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(player),
            None
        );
    }

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_007);
    let player = ObjectGuid::create_player(1, 18_007);
    let spell_id = 18_007;
    session.player_guid = Some(player);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let mut spell_store = wow_data::SpellStore::new();
    let mut attributes = [0; 15];
    attributes[2] = wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            7,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: spell_id as u32,
            difficulty_id: 0,
            base_level: 1,
            max_level: 80,
            spell_level: 12,
            max_passive_aura_level: 0,
            spell_id: spell_id as u32,
        },
    ])));

    session.apply_damage(Some(spell_id), guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(creature.current_hp(), 33);
    assert!(!creature.creature.is_in_combat());
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(player),
        None,
        "C++ ThreatManager::AddThreat rejects NO_INITIAL_THREAT while the owner is idle"
    );
}
#[tokio::test]
async fn spell_damage_without_threat_row_adds_spell_level_threat_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let creature_guid = test_creature_guid(18_009);
    let player_guid = ObjectGuid::create_player(1, 18_009);
    let spell_id = 18_009;
    session.player_guid = Some(player_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            7,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: spell_id as u32,
            difficulty_id: 0,
            base_level: 1,
            max_level: 80,
            spell_level: 12,
            max_passive_aura_level: 0,
            spell_id: spell_id as u32,
        },
    ])));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .unwrap();

    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(player_guid),
        Some(19.0),
        "C++ HandleThreatSpells falls back to SpellInfo::SpellLevel without a spell_threat row"
    );
}
#[tokio::test]
async fn spell_damage_adds_threat_without_overwriting_current_victim_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let creature_guid = test_creature_guid(18_004);
    let tank = ObjectGuid::create_player(1, 44);
    let caster = ObjectGuid::create_player(1, 45);
    let spell_id = 18_008;
    session.player_guid = Some(caster);
    let mut spell_store = wow_data::SpellStore::new();
    let mut attributes = [0; 15];
    attributes[2] = wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            7,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: spell_id as u32,
            difficulty_id: 0,
            base_level: 1,
            max_level: 80,
            spell_level: 12,
            max_passive_aura_level: 0,
            spell_id: spell_id as u32,
        },
    ])));
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(tank);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(tank, 100.0);
        })
        .unwrap();

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .unwrap();

    let manager = manager.read().unwrap();
    let creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        creature.creature.ai_ownership().combat_target,
        Some(tank),
        "C++ damage adds threat; the regular victim-selection pass decides whether to switch"
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(caster),
        Some(7.0),
        "DealDamage allows NO_INITIAL_THREAT once engaged, while HandleThreatSpells still suppresses the SpellLevel bonus"
    );
}
#[tokio::test]
async fn spell_damage_kill_keeps_empty_creature_loot_non_lootable_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_006);
    let player = ObjectGuid::create_player(1, 47);
    session.player_guid = Some(player);
    register_test_creature(&mut session, manager.clone(), guid, 40);

    session.apply_damage(None, guid, 100).await.unwrap();

    let loot = session
        .loot_table
        .get(&guid)
        .expect("creature corpse loot is generated during kill");
    assert!(loot.allowed_looters.contains(&player));
    assert_eq!(loot.loot_type, LOOT_TYPE_CORPSE_LIKE_CPP);
    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.creature.is_tapped_by(player));
    assert!(
        loot.coins == 0 && loot.unlooted_count == 0,
        "the test creature has no configured loot"
    );
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
