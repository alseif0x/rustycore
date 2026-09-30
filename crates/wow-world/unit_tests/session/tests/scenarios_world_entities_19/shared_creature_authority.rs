//! Shared creature-state and map-owner scenarios.

use super::*;

// Anchor: legacy motor A (WorldSession) shares a single Arc<RwLock<MapManager>>
// across sessions — mutations from one session are immediately visible to the
// other without any message-passing. This characterizes the shared-state
// contract that tick_creatures_sync / tick_combat_sync depend on.
#[tokio::test]
async fn two_sessions_sharing_legacy_map_manager_see_same_creature_state() {
    let manager = shared_map_manager();

    let (mut session1, _pkt_tx1, _send_rx1) = make_session();
    let (mut session2, _pkt_tx2, _send_rx2) = make_session();

    session1.set_map_manager(Arc::clone(&manager));
    session2.set_map_manager(Arc::clone(&manager));

    let creature_guid = test_creature_guid(99901);
    let pos = Position::new(10.0, 10.0, 0.0, 0.0);
    let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(pos.x, pos.y);
    manager.write().unwrap().add_creature(
        0,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            900,
            pos,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    let hp_before = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .expect("creature must exist before mutation")
        .current_hp();
    assert_eq!(hp_before, 100, "initial HP must be 100");

    session1.mutate_world_creature(creature_guid, |c| {
        c.take_damage(40);
    });

    let hp_after = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .expect("creature must exist after mutation")
        .current_hp();
    assert_eq!(
        hp_after, 60,
        "session2 must observe HP=60 after session1 applied 40 damage"
    );

    let guids2 = session2.world_creature_guids();
    assert!(
        guids2.contains(&creature_guid),
        "session2 must see creature inserted via shared manager"
    );
}
/// Two players attacking one creature resolve it once, not twice.
///
/// This is #28's acceptance criterion, and nothing in the tree asserted it: the
/// existing "two session" tests either hand-copy the owner guard or drive a
/// single session. Under `GlobalLegacy` the map owns the transition, so one
/// pass of the phase applies each attacker's swing exactly once against shared
/// creature state — and running the phase twice with no time elapsed must not
/// apply anything again, because the swing timers were consumed.
#[tokio::test]
async fn two_players_attacking_one_creature_resolve_once_under_the_map_owner_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_930);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    // Two attackers, each a real canonical player swinging at the same creature.
    let registry = PlayerRegistry::new();
    let mut attackers = Vec::new();
    for (index, counter) in [(0usize, 5_101i64), (1usize, 5_102i64)] {
        let player_guid = ObjectGuid::create_player(1, counter);
        let (mut session, _pkt_tx, _send_rx) = make_session();
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::clone(&map_store));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            player_guid,
            format!("Attacker{index}"),
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
                unit.set_attacking(Some(creature_guid));
                unit.set_target(creature_guid);
                unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
                unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
                unit.set_weapon_damage(WeaponAttackType::BaseAttack, 5.0, 5.0);
                unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
            })
            .unwrap();
        // Only the first session registers the creature; both share the manager.
        if index == 0 {
            session.set_map_manager(Arc::clone(&manager));
            register_test_creature(&mut session, manager.clone(), creature_guid, 100);
            // C++ `Unit::IsTotem()` zeroes the victim's dodge, parry and block
            // (`Unit.cpp:2313-2360`); both attackers must land their swing.
            session
                .mutate_world_creature(creature_guid, |creature| {
                    creature
                        .creature
                        .add_unit_type_mask_like_cpp(wow_entities::UNIT_MASK_TOTEM);
                })
                .unwrap();
        }
        // `PlayerRegistration` is opaque by design (#150): the only way to get one
        // is to register, which is also what production does.
        let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
        let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
        let registration = registry.register_or_replace(
            player_guid,
            broadcast_info_with_command(player_guid, send_tx, command_tx),
            Default::default(),
        );
        attackers.push(crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
            registration,
            player_guid,
            map_id: 0,
            instance_id: 0,
            in_combat_mirror: true,
            tap_group_guids: Vec::new(),
        });
    }

    let health_before = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .current_hp();

    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let first = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &crate::session::LegacyCreatureAggroConfigLikeCpp::default(),
    );

    assert!(!first.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(first.attackers_seen, 2);
    assert_eq!(
        first.victims_resolved, 2,
        "both attackers resolve the victim"
    );
    assert_eq!(
        first.creature_hits, 2,
        "each attacker lands its own swing, once"
    );

    let health_after_first = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .current_hp();
    assert!(
        health_after_first < health_before,
        "the shared creature took damage"
    );

    // A second pass with no time elapsed must apply nothing: the swing timers
    // were consumed by the first. This is the double-tick guard.
    let second = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        0,
        &mut phase_state,
        &crate::session::LegacyCreatureAggroConfigLikeCpp::default(),
    );
    assert_eq!(
        second.creature_hits, 0,
        "a second pass with no elapsed time must not resolve the same swing again"
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .current_hp(),
        health_after_first,
        "and the creature's health must not move"
    );
}
