//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_melee_tick_once_preserves_sparring_damage_clamp_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_030);
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(victim)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(52);
        typed.set_sparring_health_pct_like_cpp(50.0);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_031);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, Arc::clone(&manager), victim, 52);
    session
        .mutate_world_creature(victim, |creature| {
            creature.creature.set_sparring_health_pct_like_cpp(50.0);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .mutate_world_creature(attacker, |creature| {
            creature.enter_combat(victim);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.creature.ai_ownership_mut().min_damage = 52;
            creature.creature.ai_ownership_mut().max_damage = 52;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    assert_eq!(outcome.legacy_creature_victim_sync_cas_rejections, 0);
    assert_eq!(outcome.plan.events.len(), 2);
    let mut packet =
        wow_packet::world_packet::WorldPacket::from_bytes(&outcome.plan.events[0].packet_bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AttackerStateUpdate as u16
    );
    assert!(!packet.read_bit().unwrap());
    let info_len = packet.read_uint32().unwrap() as usize;
    let info_bytes = packet.read_bytes(info_len).unwrap();
    let mut info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
    assert_eq!(info.read_uint32().unwrap(), 0x0000_0002);
    assert_eq!(info.read_packed_guid().unwrap(), attacker);
    assert_eq!(info.read_packed_guid().unwrap(), victim);
    assert_eq!(info.read_int32().unwrap(), 52);
    assert_eq!(info.read_int32().unwrap(), 52);
    assert_eq!(
        info.read_int32().unwrap(),
        0,
        "C++ wire overdamage uses raw damage before the sparring clamp"
    );
    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 50);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, victim)
            .unwrap()
            .creature
            .unit()
            .data()
            .health,
        50
    );
}
#[test]
fn legacy_creature_melee_tick_once_preserves_fake_damage_wire_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_FAKE_DAMAGE, HIT_INFO_NORMAL_SWING};
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_032);
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(victim)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(50);
        typed.set_sparring_health_pct_like_cpp(50.0);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_033);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .mutate_world_creature(attacker, |creature| {
            creature.enter_combat(victim);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert!(!outcome.plan.events.is_empty());
    let mut packet =
        wow_packet::world_packet::WorldPacket::from_bytes(&outcome.plan.events[0].packet_bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::AttackerStateUpdate as u16
    );
    assert!(!packet.read_bit().expect("has_log_data"));
    let info_len = packet.read_uint32().expect("attackRoundInfo size") as usize;
    let info_bytes = packet.read_bytes(info_len).expect("attackRoundInfo bytes");
    let mut attack_round_info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
    assert_eq!(
        attack_round_info.read_uint32().expect("hitInfo"),
        HIT_INFO_NORMAL_SWING | HIT_INFO_FAKE_DAMAGE
    );
    let health = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap()
        .unit()
        .data()
        .health;
    assert_eq!(health, 50);
}
#[test]
fn legacy_creature_melee_tick_once_preserves_lethal_creature_outcome_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let victim = test_creature_guid(91_034);
    add_canonical_test_creature_on_map(
        &canonical,
        victim,
        9002,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(victim)
            .unwrap();
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(1);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let attacker = test_creature_guid(91_035);
    add_canonical_test_creature_on_map(
        &canonical,
        attacker,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), victim, 1);
    register_test_creature(&mut session, manager.clone(), attacker, 25);
    session
        .mutate_world_creature(attacker, |creature| {
            creature.enter_combat(victim);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(outcome.legacy_creature_victim_syncs, 1);
    assert_eq!(outcome.plan.events.len(), 2);
    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(victim, Clone::clone)
        .unwrap();
    assert_eq!(creature.unit().data().health, 0);
    assert_eq!(
        creature.unit().death_state(),
        wow_constants::DeathState::Corpse
    );
    drop(guard);
    let legacy_guard = manager.read().unwrap();
    let legacy_creature = legacy_guard.find_creature(0, 0, victim).unwrap();
    assert_eq!(legacy_creature.creature.unit().data().health, 0);
    assert_eq!(
        legacy_creature.creature.unit().death_state(),
        wow_constants::DeathState::Corpse
    );
}
#[test]
fn legacy_creature_melee_los_gate_delegates_to_map_environment_like_cpp() {
    let attacker = melee_los_test_world_object(
        test_creature_guid(91_026),
        TypeId::Unit,
        wow_constants::TypeMask::UNIT,
        Position::new(10.0, 10.0, 0.0, 0.0),
    );
    let victim = melee_los_test_world_object(
        ObjectGuid::create_player(1, 91_027),
        TypeId::Player,
        wow_constants::TypeMask::PLAYER,
        Position::new(11.0, 10.0, 0.0, 0.0),
    );

    assert!(!is_creature_melee_los_clear_like_cpp(
        &attacker,
        &victim,
        &CreatureMeleeLosTestEnvironment { los: false },
    ));
    assert!(is_creature_melee_los_clear_like_cpp(
        &attacker,
        &victim,
        &CreatureMeleeLosTestEnvironment { los: true },
    ));
}
#[test]
fn legacy_creature_melee_tick_once_rejects_charging_before_ready_swing_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_013);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_014);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::CHARGING.bits());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(rejected.creatures_seen, 1);
    assert_eq!(rejected.swings_ready, 0);
    assert_eq!(rejected.melee_precondition_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    assert!(rejected.commands.is_empty());
    {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        assert_eq!(
            creature.creature.ai_ownership().swing_timer_ms,
            0,
            "C++ returns before isAttackReady/resetAttackTimer while charging"
        );
    }
}
#[test]
fn legacy_creature_melee_tick_once_rejects_no_melee_static_flag_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_020);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_021);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            let mut static_flags = [0; 8];
            static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
            creature
                .creature
                .set_static_flags_runtime_like_cpp(static_flags);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(rejected.creatures_seen, 1);
    assert_eq!(rejected.swings_ready, 0);
    assert_eq!(rejected.melee_precondition_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    assert!(rejected.commands.is_empty());
    let guard = manager.read().unwrap();
    let creature = guard.find_creature(0, 0, creature_guid).unwrap();
    assert!(
        creature.runtime_rng_authority_complete_like_cpp(),
        "NO_MELEE returns before CalculateMeleeDamage consumes runtime RNG"
    );
    assert_eq!(creature.creature.ai_ownership().last_swing_ms, 0);
    assert_eq!(creature.creature.ai_ownership().swing_timer_ms, 0);
}
#[test]
fn legacy_creature_melee_tick_once_rejects_blocking_channel_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_015);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );

    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_016);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.creature.unit_mut().set_current_cast_spell(
                wow_entities::CurrentSpellSlot::Channeled,
                wow_entities::CurrentSpellRef::new(12_347, Some(creature_guid), None),
            );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let rejected = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(rejected.creatures_seen, 1);
    assert_eq!(rejected.swings_ready, 0);
    assert_eq!(rejected.melee_precondition_rejections, 1);
    assert_eq!(rejected.canonical_hits, 0);
    assert!(rejected.commands.is_empty());
}
#[test]
fn legacy_creature_melee_tick_once_respects_channel_allow_actions_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 91_018);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player)
            .unwrap();
        typed.unit_mut().set_max_health(100);
        typed.unit_mut().set_health(100);
    }

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let creature_guid = test_creature_guid(91_019);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.creature.unit_mut().set_current_cast_spell(
                wow_entities::CurrentSpellSlot::Channeled,
                wow_entities::CurrentSpellRef::new(12_348, Some(creature_guid), None)
                    .with_allow_actions_during_channel(true),
            );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));

    assert_eq!(outcome.melee_precondition_rejections, 0);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(outcome.melee_outcomes_unrepresented, 1);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert!(outcome.plan.events.is_empty());
    assert!(
        !manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .runtime_rng_authority_complete_like_cpp()
    );
}
#[test]
fn legacy_creature_lifecycle_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let guid = test_creature_guid(90_009);
    register_test_creature(&mut session, manager.clone(), guid, 25);

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        None,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        Instant::now(),
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert_eq!(outcome.creatures_seen, 0);
    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(outcome.respawns_processed, 0);
    assert!(outcome.refresh_map_keys.is_empty());
    assert!(
        manager.read().unwrap().find_creature(0, 0, guid).is_some(),
        "default Session owner must leave the legacy creature untouched"
    );
}
#[test]
fn legacy_creature_lifecycle_persistence_does_not_double_count_stale_tick_time_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let guid = test_creature_guid(90_020);
    register_test_creature(&mut session, Arc::clone(&manager), guid, 10);
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.set_spawn_id(90_020);
            creature.creature.ai_ownership_mut().respawn_time_secs = 30;
            assert!(creature.take_damage(10));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let before_unix = unix_now();
    let stale_tick_now = Instant::now() - Duration::from_secs(10);
    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        stale_tick_now,
    );

    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    let respawn_time = match outcome.respawn_db_mutations[0] {
        wow_persistence::RespawnPersistenceMutationLikeCpp::Save { respawn_time, .. } => {
            respawn_time
        }
        _ => panic!("the death transition must emit a typed respawn save"),
    };
    assert!(respawn_time >= before_unix + 28);
    assert!(
        respawn_time <= unix_now() + 30,
        "the stale scheduler Instant must not be added again to the Unix respawn time"
    );
}
#[test]
fn legacy_creature_lifecycle_tick_once_despawns_corpse_queues_respawn_and_refresh_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let guid = test_creature_guid(90_010);
    register_test_creature(&mut session, manager.clone(), guid, 10);

    let now = Instant::now();
    let past = now - Duration::from_secs(1);
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.set_spawn_id(90_010);
            creature.take_damage(10);
            creature.set_corpse_despawn_at(Some(past));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.corpses_despawned, 1);
    assert_eq!(outcome.respawns_processed, 0);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(
        matches!(
            outcome.respawn_db_mutations[0],
            wow_persistence::RespawnPersistenceMutationLikeCpp::Save { respawn_time, .. }
                if respawn_time > unix_now()
        ),
        "C++ persists an absolute future GameTime value, never creature-local uptime"
    );
    assert_eq!(outcome.canonical_removes, 1);
    assert_eq!(outcome.canonical_inserts, 0);
    assert_eq!(outcome.canonical_respawn_adds, 1);
    assert_eq!(outcome.canonical_respawn_removes, 0);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);

    {
        let guard = manager.read().unwrap();
        assert!(
            guard.find_creature(0, 0, guid).is_none(),
            "corpse must be removed from the legacy map"
        );
        assert_eq!(
            guard.respawn_queue_len(0, 0),
            1,
            "corpse removal must enqueue exactly one map-owned respawn"
        );
    }
    {
        let guard = canonical.lock().unwrap();
        assert!(
            guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .with_creature_like_cpp(guid, Clone::clone)
                .is_none(),
            "canonical map object must be removed outside the legacy lock"
        );
        assert!(
            guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_respawn_time_like_cpp(
                    wow_map::SpawnObjectType::Creature,
                    (guid.low_value() as u64) & 0xFF_FFFF_FFFF,
                )
                > 0,
            "canonical respawn store must retain the same timer C++ Map::SaveRespawnTime adds"
        );
    }
}
#[test]
fn legacy_creature_lifecycle_tick_once_does_not_persist_dungeon_respawn_like_cpp() {
    assert_instanceable_map_does_not_persist_respawn_like_cpp(
        33,
        77,
        wow_data::map::MAP_INSTANCE,
        0,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        90_016,
    );
}
#[test]
fn legacy_creature_lifecycle_tick_once_does_not_persist_garrison_respawn_like_cpp() {
    assert_instanceable_map_does_not_persist_respawn_like_cpp(
        1_151,
        90_019,
        wow_data::map::MAP_SCENARIO,
        wow_data::map::MAP_FLAG_GARRISON,
        // Rust currently represents garrisons as World-kind managed maps;
        // persistence must still follow C++ MapEntry::Instanceable().
        wow_map::ManagedMapKind::World,
        90_019,
    );
}
#[test]
fn legacy_creature_lifecycle_tick_once_respawns_ready_queue_and_syncs_canonical_like_cpp() {
    use crate::map_manager::{RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let now = Instant::now();
    let guid = test_creature_guid(90_011);
    let mut world_creature = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        Position::new(5.0, 6.0, 7.0, 1.0),
        25,
        3,
        4,
        8,
        20.0,
        100,
        14,
        0,
        0,
    );
    world_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(77, wow_constants::PhaseFlags::empty(), 1);
    world_creature.creature.ai_ownership_mut().phase_id = 77;
    world_creature.creature.set_spawn_id(90_011);
    let pending = pending_respawn_from_world_creature_like_cpp(
        &world_creature,
        now - Duration::from_secs(1),
        0,
    );
    {
        let grid = wow_map::compute_grid_coord(pending.home_pos.x, pending.home_pos.y);
        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .add_respawn_info_like_cpp(wow_map::RespawnInfoLikeCpp {
                object_type: wow_map::SpawnObjectType::Creature,
                spawn_id: pending.spawn_id,
                entry: pending.create_data.entry,
                respawn_time: unix_now() + 1,
                grid_id: grid.get_id(),
            });
    }
    {
        let mut guard = manager.write().unwrap();
        guard.save_pending_respawn_time_like_cpp(0, 0, &pending, now, unix_now());
        guard.push_respawn(0, 0, pending);
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now,
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.corpses_despawned, 0);
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.respawn_db_mutations.len(), 1);
    assert!(matches!(
        outcome.respawn_db_mutations[0],
        wow_persistence::RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));
    assert_eq!(outcome.canonical_removes, 0);
    assert_eq!(outcome.canonical_inserts, 1);
    assert_eq!(outcome.canonical_respawn_adds, 0);
    assert_eq!(outcome.canonical_respawn_removes, 1);
    assert_eq!(outcome.refresh_map_keys, vec![(0, 0)]);

    {
        let guard = manager.read().unwrap();
        let creature = guard
            .find_creature(0, 0, guid)
            .expect("ready respawn must re-add the legacy creature");
        assert_eq!(creature.position(), Position::new(5.0, 6.0, 7.0, 1.0));
        assert!(
            creature.phase_shift().has_phase_like_cpp(77),
            "resolved phase shift must survive session-free respawn"
        );
        assert_eq!(guard.respawn_queue_len(0, 0), 0);
    }
    {
        let guard = canonical.lock().unwrap();
        let typed = guard
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(guid, Clone::clone)
            .expect("canonical creature must be inserted outside the legacy lock");
        assert!(
            typed.unit().world().phase_shift().has_phase_like_cpp(77),
            "canonical respawn must preserve the captured phase shift"
        );
        assert_eq!(
            guard
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_respawn_time_like_cpp(
                    wow_map::SpawnObjectType::Creature,
                    (guid.low_value() as u64) & 0xFF_FFFF_FFFF,
                ),
            0,
            "ready respawn must clear the canonical timer before the grid can reload stale state"
        );
    }
}
