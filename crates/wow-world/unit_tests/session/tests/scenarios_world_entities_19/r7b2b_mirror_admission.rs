//! F6-7 R7b-2b regressions: the shared map-level mirror gate.
//!
//! R7a gated the single mutation root; R7b-2a gated the remaining mutation
//! roots. The two production **mirror** sites — the player melee tick and the
//! creature movement tick — still called
//! `sync_canonical_creature_entity_on_map_like_cpp` directly and ran the legacy
//! authority compare-and-exchange themselves. That return value is `Some` for an
//! **applied or rejected** snapshot, and the application returns `Rejected` for
//! a representation the incarnation admitted but whose entity state it refused
//! (a stale or equal-revision-mismatched claim). So the old mirror sites
//! rebound the legacy loot alias onto the incarnation's authority for a
//! snapshot the canonical incarnation never applied, and the player site did not
//! count it as a rejection.
//!
//! R7b-2b moves the whole sequence into one shared map-level gate,
//! `sync_admitted_creature_representation_on_map_like_cpp`, which evaluates the
//! R7a/R7b-2a admission predicate against the current incarnation **before**
//! invoking the application, applies the representation under the canonical
//! guard in canonical→legacy lock order, rebinds the legacy alias only for an
//! applied snapshot, and returns `true` if and only if the canonical incarnation
//! applied the representation. A missing legacy alias or a failed alias CAS does
//! not change that.
//!
//! These tests drive the production root and the two production tick sites.

use super::r7a_canonical_mutation::*;
use super::r7b2a_canonical_mutation::*;
use super::*;

fn test_position_like_cpp() -> Position {
    Position::new(10.0, 10.0, 0.0, 0.0)
}

/// Queue one ready respawn for `guid` and run the production global lifecycle
/// tick once. Mirrors the R1b fixture; kept local so this slice owns its
/// fixtures.
fn run_respawn_tick_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: Option<&SharedCanonicalMapManager>,
    guid: ObjectGuid,
    hp: u32,
    now: Instant,
) -> crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp {
    use crate::map_manager::{RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp};

    let queued = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        test_position_like_cpp(),
        hp,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    let pending = pending_respawn_from_world_creature_like_cpp(&queued, now, 0);
    {
        let mut guard = manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        guard.push_respawn(0, 0, pending);
    }
    run_legacy_creature_lifecycle_tick_once_like_cpp(
        manager,
        canonical,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now + Duration::from_secs(1),
    )
}

/// The pristine-duplicate respawn the sibling R1b regressions use: a canonical
/// incarnation for the GUID already exists while the legacy store has none, so
/// the ready respawn must adopt the admitted incarnation's authority and health
/// timeline before it becomes claimable.
fn setup_pristine_duplicate_respawn_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    canonical_health: u64,
    respawn_health: u32,
    now: Instant,
) -> crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp {
    add_canonical_test_creature_on_map(canonical, guid, 9001, test_position_like_cpp(), 0, 0, 0);
    {
        let mut guard = canonical.lock().unwrap();
        let typed = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .expect("the canonical incarnation pre-exists");
        typed.unit_mut().set_level(80);
        typed.unit_mut().set_max_health(canonical_health);
        typed.unit_mut().set_health(canonical_health);
    }
    run_respawn_tick_like_cpp(manager, Some(canonical), guid, respawn_health, now)
}

/// One mirror attempt exactly as the tick sites make it: the transported
/// representation is the legacy clone and the expectation is that clone's own
/// allocation/stamp.
fn attempt_mirror_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    transported: wow_entities::Creature,
) -> bool {
    let expected = transported.loot_authority_like_cpp().clone();
    let expected_stamp = expected.stamp_like_cpp();
    sync_admitted_creature_representation_on_map_like_cpp(
        canonical,
        Some(manager),
        0,
        0,
        transported,
        &expected,
        expected_stamp,
    )
}

/// One displaced incarnation pair: the canonical incarnation owns a **used**
/// allocation and the legacy representation still carries an **unused pristine
/// candidate**, so an admitted mirror is observable as the alias moving onto the
/// incarnation's allocation.
fn split_allocation_pair_like_cpp(
    guid: ObjectGuid,
    player: ObjectGuid,
) -> (
    WorldSession,
    crate::map_manager::SharedMapManager,
    SharedCanonicalMapManager,
    wow_loot::OwnedLootAuthority,
    wow_loot::OwnedLootAuthority,
) {
    let (session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let pristine = wow_loot::OwnedLootAuthority::new();
    let incarnation = observed_fully_looted_authority_like_cpp(guid, player).0;
    rebind_canonical_authority_like_cpp(&canonical, guid, incarnation.clone());
    rebind_legacy_authority_like_cpp(&manager, guid, pristine.clone());
    (session, manager, canonical, pristine, incarnation)
}

/// The whole R7b-2b map-gate contract in one regression: fresh acceptance, every
/// refusal class, that a refusal neither applies nor rebinds, that the absence of
/// a legacy representation still permits the canonical application, and that a
/// stale alias CAS preserves the newer alias.
#[test]
fn admitted_map_mirror_preserves_incarnation_and_expected_stamp_contract_like_cpp() {
    // ---- fresh acceptance: the canonical incarnation applies and the alias
    // is moved off the displaced candidate onto the incarnation's allocation.
    let guid = test_creature_guid(92_500);
    let player = ObjectGuid::create_player(1, 92_501);
    let (_session, manager, canonical, pristine, incarnation) =
        split_allocation_pair_like_cpp(guid, player);
    let incarnation_stamp = incarnation.stamp_like_cpp();
    let mut transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        transported
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "the transported representation carries the displaced candidate"
    );
    transported.ai_ownership_mut().npc_flags = 0x0001_00C1;
    assert!(
        attempt_mirror_like_cpp(&manager, &canonical, transported),
        "a fresh same-incarnation representation is admitted and applied"
    );
    let owner = canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation");
    assert_eq!(
        owner.ai_ownership().npc_flags,
        0x0001_00C1,
        "the canonical incarnation carries the transported state"
    );
    assert!(
        owner
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the incarnation keeps the allocation it owns"
    );
    assert_eq!(
        owner.loot_authority_like_cpp().stamp_like_cpp(),
        incarnation_stamp,
        "an admitted pristine candidate does not move the incarnation's stamp"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the legacy alias is synchronized onto the incarnation's allocation"
    );
    assert!(
        !legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "the alias no longer holds the displaced candidate"
    );
    assert_one_incarnation_like_cpp(&manager, &canonical, guid);

    // ---- stale revision: the incarnation advanced without the representation.
    let guid = test_creature_guid(92_502);
    let player = ObjectGuid::create_player(1, 92_503);
    let (_session, manager, canonical, pristine, _) = split_allocation_pair_like_cpp(guid, player);
    advance_canonical_max_health_like_cpp(&canonical, guid, 140);
    let stale = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);
    assert!(
        !attempt_mirror_like_cpp(&manager, &canonical, stale),
        "a stale representation is refused before the application"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "a refused mirror applies nothing and rebinds nothing"
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "a refused mirror does not move the legacy alias"
    );

    // ---- equal revision with a mismatched health tuple is refused too.
    let guid = test_creature_guid(92_504);
    let player = ObjectGuid::create_player(1, 92_505);
    let (_session, manager, canonical, pristine, _) = split_allocation_pair_like_cpp(guid, player);
    advance_canonical_max_health_like_cpp(&canonical, guid, 140);
    let owner = canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation");
    let current_revision = owner.unit().health_state_revision_like_cpp();
    assert_ne!(current_revision, 0, "the incarnation has a health timeline");
    let mut transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    transported
        .unit_mut()
        .set_max_health(owner.unit().data().max_health + 10);
    transported
        .unit_mut()
        .adopt_committed_health_state_revision_for_mirror_like_cpp(current_revision);
    assert_eq!(
        transported.unit().health_state_revision_like_cpp(),
        current_revision,
        "the transported claim sits at the incarnation's revision"
    );
    assert_ne!(
        transported.unit().data().max_health,
        owner.unit().data().max_health
    );
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);
    assert!(
        !attempt_mirror_like_cpp(&manager, &canonical, transported),
        "an equal-revision tuple mismatch is not the incarnation's represented state"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine)
    );

    // ---- ABA: health leaves and returns to the same tuple while the revision
    // advances, so the transported tuple agrees but is stale.
    let guid = test_creature_guid(92_506);
    let player = ObjectGuid::create_player(1, 92_507);
    let (_session, manager, canonical, pristine, _) = split_allocation_pair_like_cpp(guid, player);
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(0, 0).expect("canonical map instance");
        map.map_mut()
            .with_creature_mut_like_cpp(guid, |creature| {
                creature.unit_mut().set_health(60);
                creature.unit_mut().set_health(100);
            })
            .expect("canonical incarnation");
    }
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);
    assert!(
        !attempt_mirror_like_cpp(&manager, &canonical, transported),
        "an ABA replay is a stale representation, not a fresh one"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine)
    );

    // ---- replacement incarnation under the same GUID.
    let guid = test_creature_guid(92_508);
    let (_session, manager, canonical, pristine, _) =
        split_allocation_pair_like_cpp(guid, ObjectGuid::EMPTY);
    let destroyed = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    remove_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, guid);
    let replacement = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        0,
        0,
        admission_candidate_like_cpp(guid),
    )
    .expect("the recreation is admitted");
    let replacement_before = loot_lifecycle_state_like_cpp(
        &canonical_creature_like_cpp(&canonical, guid).expect("replacement"),
    );
    assert!(
        !attempt_mirror_like_cpp(&manager, &canonical, destroyed),
        "a representation of a destroyed incarnation must not reach the replacement"
    );
    assert_eq!(
        loot_lifecycle_state_like_cpp(
            &canonical_creature_like_cpp(&canonical, guid).expect("replacement")
        ),
        replacement_before,
        "the replacement keeps its own state and lifetime"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("replacement")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement.loot_authority)
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine)
    );

    // ---- competing used allocation for the same incarnation.
    let guid = test_creature_guid(92_509);
    let player = ObjectGuid::create_player(1, 92_510);
    let (_session, manager, canonical, _, _) = split_allocation_pair_like_cpp(guid, player);
    let competing = observed_fully_looted_authority_like_cpp(guid, player).0;
    rebind_legacy_authority_like_cpp(&manager, guid, competing.clone());
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);
    assert!(
        !attempt_mirror_like_cpp(&manager, &canonical, transported),
        "a competing used allocation must not be reconciled into the incarnation"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "the refused representation keeps its own competing allocation"
    );
    assert_eq!(
        competing.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active,
        "the competing pool is neither quarantined nor retired"
    );

    // ---- missing canonical map instance, then missing incarnation.
    let guid = test_creature_guid(92_511);
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    register_test_creature(&mut session, manager.clone(), guid, 100);
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = loot_lifecycle_state_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    let empty = shared_canonical_map_manager();
    assert!(
        !attempt_mirror_like_cpp(&manager, &empty, transported.clone()),
        "a missing canonical map instance has no incarnation to admit the representation"
    );
    empty.lock().unwrap().create_world_map(0, 0);
    assert!(
        !attempt_mirror_like_cpp(&manager, &empty, transported.clone()),
        "a missing canonical incarnation is unavailable ownership"
    );
    assert!(
        canonical_creature_like_cpp(&empty, guid).is_none(),
        "every refusal created no incarnation"
    );
    assert_eq!(
        loot_lifecycle_state_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        before,
        "every refusal left the representation untouched"
    );

    // ---- a missing legacy representation still permits the canonical
    // application: the canonical incarnation is the ownership this root
    // resolves, and the absent alias is a CAS with nothing to update.
    let guid = test_creature_guid(92_512);
    let (_session, manager, canonical, _, _) =
        split_allocation_pair_like_cpp(guid, ObjectGuid::EMPTY);
    let mut transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    transported.ai_ownership_mut().npc_flags = 0x0001_00C2;
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove_creature_any(0, 0, guid)
        .expect("the fixture registered the legacy representation");
    assert!(
        attempt_mirror_like_cpp(&manager, &canonical, transported.clone()),
        "the canonical incarnation is the ownership this root resolves"
    );
    assert_eq!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .ai_ownership()
            .npc_flags,
        0x0001_00C2,
        "the canonical application happened with no alias to synchronize"
    );

    // ---- a stale alias CAS preserves the newer alias while the canonical
    // application is still reported as applied.
    let guid = test_creature_guid(92_513);
    let player = ObjectGuid::create_player(1, 92_514);
    let (_session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let newer = wow_loot::OwnedLootAuthority::new();
    rebind_legacy_authority_like_cpp(&manager, guid, newer.clone());
    assert!(
        attempt_mirror_like_cpp(&manager, &canonical, transported),
        "the canonical incarnation applied the representation"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&newer),
        "a failed alias compare-and-exchange preserves the newer alias"
    );
    assert!(
        !legacy.loot_authority_like_cpp().shares_storage_like_cpp(
            canonical_creature_like_cpp(&canonical, guid)
                .expect("canonical incarnation")
                .loot_authority_like_cpp()
        ),
        "the newer alias is not displaced by a snapshot the alias did not expect"
    );
    let _ = player;
}

/// **Negative control (R7b-2b primary).** This drives the real player melee tick
/// mirror against a representation whose claimed incarnation is no longer the
/// current one: the canonical object advanced its health timeline and owns a used
/// allocation while the representation still carries an unused candidate, so the
/// incarnation is foreign to that claim. The swing is forced to be avoided, so
/// the tick leaves the legacy entity untouched and the transported snapshot
/// really is the displaced claim. Against the pre-slice body the tick still
/// rebounds the legacy alias onto the incarnation's allocation and reports no
/// rejection; with the shared gate the mirror is refused, the canonical state and
/// authority are unchanged and the alias keeps its own allocation.
#[tokio::test]
async fn player_melee_tick_refuses_foreign_incarnation_mirror_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let victim = test_creature_guid(92_600);
    let outcome = setup_pristine_duplicate_respawn_like_cpp(
        &manager,
        &canonical,
        victim,
        100_000,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);

    // The incarnation is displaced: it owns a used allocation and a newer health
    // state, while the representation still carries an unused candidate.
    let player_guid = ObjectGuid::create_player(1, 92_600);
    let pristine = wow_loot::OwnedLootAuthority::new();
    let incarnation = observed_fully_looted_authority_like_cpp(victim, player_guid).0;
    rebind_legacy_authority_like_cpp(&manager, victim, pristine.clone());
    rebind_canonical_authority_like_cpp(&canonical, victim, incarnation.clone());
    advance_canonical_max_health_like_cpp(&canonical, victim, 100_500);
    // The swing is avoided, so the legacy entity is not mutated and the mirror
    // transports the displaced claim itself.
    {
        let mut guard = manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature = guard
            .find_creature_mut(0, 0, victim)
            .expect("legacy representation");
        let mut avoidance = creature.creature.avoidance_like_cpp();
        avoidance.dodge_pct = 100.0;
        creature.creature.set_avoidance_like_cpp(avoidance);
    }

    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "R7b2bSolo".to_string(),
        test_position_like_cpp(),
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
            unit.set_attacking(Some(victim));
            unit.set_target(victim);
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
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let canonical_before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, victim).1;

    // The represented attack table is only resolved when the config carries a
    // spell store, so the forced avoidance needs one. It is empty on purpose:
    // every aura term stays zero and the victim's own dodge band decides.
    let melee_config = LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::new(wow_data::SpellStore::new())),
        ..Default::default()
    };
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let player_melee = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &melee_config,
    );

    assert_eq!(
        player_melee.creature_hits, 1,
        "the avoided swing still resolves as one hit on the victim"
    );
    assert_eq!(
        player_melee.canonical_mirror_rejections, 1,
        "the displaced claim is refused and counted exactly once"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, victim).1,
        canonical_before,
        "the refused mirror leaves the canonical state and authority untouched"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, victim)
            .expect("canonical incarnation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the incarnation keeps the allocation it owns"
    );
    let legacy = legacy_creature_like_cpp(&manager, victim).expect("legacy representation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "a refused mirror does not rebind the legacy alias"
    );
    assert!(
        !legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the legacy alias was not pointed at a snapshot the incarnation refused"
    );
}

/// **Negative control (R7b-2b, movement site).** The same displaced incarnation
/// claim through the real creature movement tick. `canonical_syncs` stays an
/// **attempt** count: the queued snapshot is processed while a canonical manager
/// exists, so it is counted even though the gate refused it.
#[test]
fn creature_movement_tick_refuses_foreign_incarnation_mirror_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let guid = test_creature_guid(92_700);
    let outcome = setup_pristine_duplicate_respawn_like_cpp(
        &manager,
        &canonical,
        guid,
        137,
        105,
        now - Duration::from_secs(1),
    );
    assert_eq!(outcome.respawns_processed, 1);

    // The incarnation is displaced exactly as in the melee case: it owns a used
    // allocation and a newer health state while the representation still carries
    // an unused candidate.
    let player = ObjectGuid::create_player(1, 92_701);
    let pristine = wow_loot::OwnedLootAuthority::new();
    let incarnation = observed_fully_looted_authority_like_cpp(guid, player).0;
    rebind_legacy_authority_like_cpp(&manager, guid, pristine.clone());
    rebind_canonical_authority_like_cpp(&canonical, guid, incarnation.clone());
    advance_canonical_max_health_like_cpp(&canonical, guid, 200);
    let canonical_before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid).1;
    let canonical_ai_before = canonical_creature_like_cpp(&canonical, guid)
        .expect("canonical incarnation")
        .ai_state();

    {
        let mut guard = manager.write().unwrap();
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        let creature = guard
            .find_creature_mut(0, 0, guid)
            .expect("legacy representation");
        creature
            .creature
            .set_default_movement_type_runtime_like_cpp(
                wow_entities::MovementGeneratorType::Random,
            );
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
        creature.seed_runtime_rng_like_cpp(0x9130);
        creature.backdate_runtime_clock_for_test(Duration::from_millis(10));
    }

    let mmap_config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let movement = run_legacy_creature_movement_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &mmap_config,
        None,
        &HashMap::new(),
        10,
    );

    assert_eq!(
        movement.canonical_syncs, 1,
        "the queued snapshot is an attempt and is counted even when refused"
    );
    // #1263 F6-8B: the same refusal now also decides the movement publication.
    // The tick keeps driving the creature's canonical runtime state, but the
    // legacy copy publishes no movement frame the canonical authority refused.
    assert_eq!(
        movement.movement_packets, 0,
        "a refused canonical application publishes no movement from the legacy copy"
    );
    assert!(
        movement.plan.events.is_empty(),
        "a refused representation decides nothing"
    );
    assert_eq!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .ai_state(),
        canonical_ai_before,
        "the refused mirror publishes no movement state to the incarnation"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid).1,
        canonical_before,
        "the refused mirror leaves the canonical state and authority untouched"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the incarnation keeps the allocation it owns"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "a refused mirror does not rebind the legacy alias"
    );
    assert!(
        !legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the legacy alias was not pointed at a snapshot the incarnation refused"
    );
}

/// Decode the update blocks of one `SMSG_UPDATE_OBJECT` packet so a CREATE can be
/// attributed to a GUID instead of to a byte length.
fn update_object_blocks_like_cpp(bytes: &[u8]) -> Vec<(u8, ObjectGuid)> {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    if packet.read_uint16().unwrap() != ServerOpcodes::UpdateObject as u16 {
        return Vec::new();
    }
    let count = packet.read_uint32().unwrap();
    let _map_id = packet.read_uint16().unwrap();
    if packet.read_bit().unwrap() {
        let _destroy_count = packet.read_uint16().unwrap();
        let total = packet.read_uint32().unwrap();
        for _ in 0..total {
            packet.read_packed_guid().unwrap();
        }
    }
    let _size = packet.read_uint32().unwrap();
    let mut blocks = Vec::new();
    for _ in 0..count {
        let update_type = packet.read_uint8().unwrap();
        let guid = packet.read_packed_guid().unwrap();
        blocks.push((update_type, guid));
        break;
    }
    blocks
}

/// F6-7 R7b-2b, deferred-window boundary. `run_creatures_tick` retries the map's
/// spawn queue every tick, and while a canonical manager is configured without a
/// canonical instance for the key the candidate is legitimately **deferred** —
/// yet the session-owned publication still announces it. This measures that
/// window instead of pinning it: across two consecutive ticks the still-deferred
/// candidate is announced once per tick, is visible, waits in one queue entry and
/// has no representation in either store, and its mutation is refused with the
/// callback unexecuted. Once the canonical instance legitimately exists the next
/// tick admits both aliases and drains the queue, and the following tick emits no
/// repeat CREATE.
///
/// Declared boundary: session-owned respawn may announce a still-deferred
/// candidate on successive retry cycles until legitimate admission; one observed
/// cycle is not a lifetime bound. R7b-2b measures and retains this publication
/// discrepancy.
#[test]
fn session_creatures_tick_deferred_spawn_window_is_measured_like_cpp() {
    let guid = test_creature_guid(92_800);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    session.set_map_manager(manager.clone());
    session.set_player_map_position_like_cpp(0, test_position_like_cpp());
    session.set_canonical_map_manager(Arc::clone(&canonical));
    assert!(
        canonical.lock().unwrap().find_map(0, 0).is_none(),
        "the fixture starts without a canonical map instance"
    );

    // Production registration at the key the legacy store publishes to.
    session.register_world_creature(
        0,
        test_position_like_cpp(),
        test_creature_create_data(guid, 9_001, 100),
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
    assert_eq!(
        manager.read().unwrap().respawn_queue_len(0, 0),
        1,
        "the candidate with no admitted owner waits in the map spawn queue"
    );
    assert!(legacy_creature_like_cpp(&manager, guid).is_none());
    assert!(canonical_creature_like_cpp(&canonical, guid).is_none());

    // A deferred candidate has no admitted incarnation: the mutation root refuses
    // before the callback runs.
    let mut callback_runs = 0_usize;
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0001_00C3;
            })
            .is_none(),
        "a deferred candidate has no admitted incarnation to mutate"
    );
    assert_eq!(callback_runs, 0, "the callback does not run");

    // Two consecutive ticks with no canonical instance: the window is measured,
    // not bounded — one CREATE for the GUID per tick, still deferred, still
    // unpublished in both stores.
    for tick in 0..2_usize {
        let output = session.run_creatures_tick();
        let mut creates_for_guid = 0_usize;
        for packet in &output.packets {
            for (update_type, block_guid) in update_object_blocks_like_cpp(packet) {
                if block_guid == guid
                    && update_type == wow_packet::packets::update::UpdateType::CreateObject as u8
                {
                    creates_for_guid += 1;
                }
            }
        }
        assert_eq!(
            creates_for_guid, 1,
            "tick {tick} announces the still-deferred candidate exactly once"
        );
        assert!(
            session.core.client_visible_guids_like_cpp.contains(&guid),
            "tick {tick} publishes the candidate into the visible set"
        );
        assert_eq!(
            manager.read().unwrap().respawn_queue_len(0, 0),
            1,
            "tick {tick} leaves the candidate waiting for legitimate admission"
        );
        assert!(
            legacy_creature_like_cpp(&manager, guid).is_none(),
            "tick {tick} publishes no legacy representation"
        );
        assert!(
            canonical_creature_like_cpp(&canonical, guid).is_none(),
            "tick {tick} admits no canonical incarnation"
        );
        session.flush_runtime_output(output);
    }

    // Legitimate admission: the instance now exists, so the next tick publishes
    // both aliases and drains the queue.
    canonical.lock().unwrap().create_world_map(0, 0);
    let admitted_tick = session.run_creatures_tick();
    let mut admitted_creates = 0_usize;
    for packet in &admitted_tick.packets {
        for (update_type, block_guid) in update_object_blocks_like_cpp(packet) {
            if block_guid == guid
                && update_type == wow_packet::packets::update::UpdateType::CreateObject as u8
            {
                admitted_creates += 1;
            }
        }
    }
    assert_eq!(
        admitted_creates, 1,
        "the admission tick still announces the candidate once"
    );
    assert_eq!(
        manager.read().unwrap().respawn_queue_len(0, 0),
        0,
        "the admitted candidate is no longer waiting"
    );
    session.flush_runtime_output(admitted_tick);
    let canonical_creature =
        canonical_creature_like_cpp(&canonical, guid).expect("the incarnation is admitted");
    let legacy_creature = legacy_creature_like_cpp(&manager, guid)
        .expect("the deferred candidate is published once it can be admitted");
    assert!(
        legacy_creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_creature.loot_authority_like_cpp()),
        "the published representation aliases the admitted incarnation's one allocation"
    );
    assert!(
        legacy_creature
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &canonical_creature
                    .unit()
                    .health_state_revision_authority_like_cpp()
            ),
        "the published representation adopts the admitted incarnation's health timeline"
    );
    assert!(
        session
            .mutate_world_creature(guid, |creature| {
                callback_runs += 1;
                creature.creature.ai_ownership_mut().npc_flags = 0x0001_00C3;
            })
            .is_some(),
        "an admitted creature is mutable"
    );
    assert_eq!(callback_runs, 1);

    // The following tick has nothing left to announce: the queue is empty and no
    // repeat CREATE is emitted for this GUID.
    let following_tick = session.run_creatures_tick();
    let mut repeat_creates = 0_usize;
    for packet in &following_tick.packets {
        for (update_type, block_guid) in update_object_blocks_like_cpp(packet) {
            if block_guid == guid
                && update_type == wow_packet::packets::update::UpdateType::CreateObject as u8
            {
                repeat_creates += 1;
            }
        }
    }
    assert_eq!(
        repeat_creates, 0,
        "an admitted candidate is not announced again on the following tick"
    );
    assert_eq!(manager.read().unwrap().respawn_queue_len(0, 0), 0);
}
