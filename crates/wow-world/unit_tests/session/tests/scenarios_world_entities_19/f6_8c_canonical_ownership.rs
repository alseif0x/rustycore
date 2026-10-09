//! #1263 F6-8C regressions: the canonical designated owner decides.
//!
//! F6-8A moved the persistent creature runtime state into canonical
//! `Creature` ownership and F6-8B gated movement/visibility publication on the
//! canonical application. C transfers the **decision source** of the combat
//! consumers: with a canonical store configured, the canonical incarnation is
//! the designated owner of a creature object, and a surviving legacy
//! representation without one decides nothing — the same designated-owner rule
//! F6-7 R2 gave the loot-authority lookup and F6-8A/B gave respawn and movement
//! publication.
//!
//! Each test drives the production tick with **one** creature: first registered
//! in the legacy store alone while the canonical store is configured but holds
//! no incarnation (refusal), then with the canonical owner holding that
//! incarnation (decision). Nothing else differs between the two halves, so the
//! gate is the only variable.

use super::*;

/// Give the canonical owner the incarnation for one legacy representation.
///
/// The fixture inserts the canonical record directly, exactly as the loot
/// fixtures do: the shared application root only synchronizes an **existing**
/// incarnation (a missing one is `Refused`, not created), and the production
/// path that creates one is the session's own registration/publication.
fn adopt_canonical_incarnation_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) {
    let representation = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .find_creature(0, 0, guid)
        .expect("the fixture registered the legacy representation")
        .creature
        .clone();
    let mut guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = guard
        .find_map_mut(0, 0)
        .expect("the fixture attached the canonical map instance");
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_creature(representation)
                .expect("the registered creature is a valid canonical record"),
        )
        .expect("the fixture inserts the canonical incarnation");
    assert!(
        map.map().with_creature_like_cpp(guid, |_| ()).is_some(),
        "the canonical owner now holds the incarnation"
    );
}

#[test]
fn legacy_creature_aggro_decides_only_through_the_canonical_incarnation_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .create_world_map(0, 0);
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(93_100);
    let player = ObjectGuid::create_player(1, 93_101);
    // Legacy-only registration: the canonical owner holds no incarnation.
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];
    let config = legacy_aggro_hostile_config_with_rate_like_cpp(3.0);

    let refused = run_legacy_creature_aggro_tick_once_with_config_and_canonical_like_cpp(
        &manager,
        Some(&canonical),
        &candidates,
        config.clone(),
    );
    assert_eq!(
        refused.canonical_incarnation_rejections, 1,
        "a legacy representation with no canonical incarnation is not the owner's object"
    );
    assert_eq!(
        refused.aggro_starts, 0,
        "a legacy-only copy decides nothing by itself"
    );
    assert!(
        refused.commands.is_empty() && refused.plan.events.is_empty(),
        "the refusal publishes nothing"
    );
    assert_eq!(
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(0, 0, creature_guid)
            .expect("the legacy representation is still registered")
            .creature
            .ai_ownership()
            .combat_target,
        None,
        "the refused scan leaves the legacy copy out of combat"
    );

    adopt_canonical_incarnation_like_cpp(&manager, &canonical, creature_guid);

    let decided = run_legacy_creature_aggro_tick_once_with_config_and_canonical_like_cpp(
        &manager,
        Some(&canonical),
        &candidates,
        config,
    );
    assert_eq!(
        decided.canonical_incarnation_rejections, 0,
        "the same representation decides once the canonical owner holds it"
    );
    assert_eq!(decided.aggro_starts, 1, "{decided:?}");
    assert_eq!(decided.commands.len(), 1);
    assert_eq!(decided.commands[0].victim_guid, player);
}

#[test]
fn legacy_creature_spell_selection_decides_only_through_the_canonical_incarnation_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(93_200);
    let victim_guid = ObjectGuid::create_player(1, 93_201);
    let spell_id = 15_691_i32;
    // Legacy-only registration first, so the canonical store is configured
    // afterwards and holds no incarnation yet.
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("CombatAI", String::new());
            creature.creature.set_spell(0, spell_id as u32);
            creature.enter_combat(victim_guid);
        })
        .unwrap();
    add_canonical_test_player_on_map(
        &canonical,
        victim_guid,
        Position::new(11.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );

    let refused = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(
        refused.canonical_incarnation_rejections, 1,
        "a legacy representation with no canonical incarnation selects no spell"
    );
    assert_eq!(refused.schedules_initialized, 0);
    assert_eq!(refused.casts_ready, 0);
    assert!(
        refused.plan.events.is_empty(),
        "the refusal publishes nothing"
    );

    adopt_canonical_incarnation_like_cpp(&manager, &canonical, creature_guid);

    let decided = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(decided.canonical_incarnation_rejections, 0);
    assert_eq!(
        decided.schedules_initialized, 1,
        "the canonical owner's creature selects its template spell"
    );
}

#[test]
fn legacy_creature_melee_selection_decides_only_through_the_canonical_incarnation_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 93_301);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let victim = guard
            .find_map_mut(0, 0)
            .expect("the fixture attached the canonical map")
            .map_mut()
            .get_typed_player_mut(player)
            .expect("the fixture attached the canonical player");
        victim.unit_mut().set_max_health(100);
        victim.unit_mut().set_health(100);
    }
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(93_302);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(0x93_302);
        })
        .unwrap();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let refused = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(
        refused.canonical_incarnation_rejections, 1,
        "a legacy representation with no canonical incarnation selects no swing"
    );
    assert_eq!(refused.swings_ready, 0);
    assert!(refused.commands.is_empty());

    adopt_canonical_incarnation_like_cpp(&manager, &canonical, creature_guid);

    let decided = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(decided.canonical_incarnation_rejections, 0);
    assert_eq!(decided.swings_ready, 1);
    assert_eq!(
        decided.attacker_incarnation_rejections, 0,
        "the canonical owner's incarnation is the one the swing is validated against"
    );
}

/// The player-melee *execution* also belongs to the canonical owner: a creature
/// victim that only the legacy store holds no longer resolves a swing.
#[tokio::test]
async fn player_melee_execution_decides_only_through_the_canonical_incarnation_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 93_400);
    let victim_guid = test_creature_guid(93_401);
    let (mut session, _pkt_tx, _send_rx) = make_session();
    // Legacy-only registration: the canonical owner holds no incarnation.
    register_test_creature(&mut session, manager.clone(), victim_guid, 100);
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
        player_guid,
        "F68CSolo".to_string(),
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
            unit.set_attacking(Some(victim_guid));
            unit.set_target(victim_guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1_000.0, 1_000.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();

    let refused = run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &LegacyCreatureAggroConfigLikeCpp::default(),
    );
    assert_eq!(
        refused.canonical_incarnation_rejections, 1,
        "a legacy-only creature victim is not the canonical owner's object"
    );
    assert_eq!(
        refused.swings_ready, 1,
        "the attacker's swing was resolved; only the victim execution is refused"
    );
    assert_eq!(refused.creature_hits, 0);
    assert!(
        refused
            .commands
            .iter()
            .all(|command| command.swings.is_empty()),
        "the refusal executes no swing against the legacy copy"
    );
}
