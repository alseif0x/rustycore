//! #1263 F6-8D3b-2 regressions of the canonical-first creature root (since
//! #1263 F6-8E the canonical map is the only creature store).
//!
//! * The session mutation root is canonical-first: a registration leaves the
//!   canonical incarnation as the creature's only representation, and a
//!   session mutation after the executor changed the incarnation's health is
//!   applied to it — the D3b-1 failure mode was a `Revision` refusal there,
//!   which left a killed creature at 0 HP with no JUST_DIED.
//! * R7b-1: a registration no canonical instance can admit yet is held on the
//!   canonical side and joins that map's respawn queue when the instance is
//!   created.

use super::*;

fn d3b2_session_like_cpp() -> (
    WorldSession,
    SharedCanonicalMapManager,
    SharedCanonicalMapManager,
) {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    (session, manager, canonical)
}

fn d3b2_register_like_cpp(session: &mut WorldSession, map_id: u16, guid: ObjectGuid) {
    session.register_world_creature(
        map_id,
        Position::new(30.0, 30.0, 0.0, 0.0),
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
}

#[test]
fn canonical_first_root_applies_a_session_mutation_after_an_executor_health_change_like_cpp() {
    let (mut session, manager, canonical) = d3b2_session_like_cpp();
    let guid = test_creature_guid(97_000);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), guid, 25);
    // The executor's player-melee phase lowers the incarnation's health in
    // place (a new health-state revision the legacy-first root refused on).
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(guid, |creature| creature.unit_mut().set_health(5))
        .expect("canonical incarnation");

    let mut runs = 0_usize;
    let applied = session.mutate_world_creature(guid, |creature| {
        runs += 1;
        creature.creature.ai_ownership_mut().npc_flags = 0x42;
        creature.creature.current_hp()
    });

    assert_eq!(applied, Some(5), "the mutation sees the executor's health");
    assert_eq!(runs, 1);
    let (health, npc_flags) = canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, |creature| {
            (creature.current_hp(), creature.ai_ownership().npc_flags)
        })
        .expect("canonical incarnation");
    assert_eq!((health, npc_flags), (5, 0x42));
    assert_eq!(
        session
            .core
            .read_world_creature_like_cpp(guid, |creature| creature.current_hp()),
        Some(5),
        "reads are canonical-first too"
    );
}

#[test]
fn r7b1_deferred_registration_waits_on_the_canonical_side_like_cpp() {
    let (mut session, manager, canonical) = d3b2_session_like_cpp();
    session.set_canonical_map_manager(manager.clone());
    let guid = test_creature_guid(97_010);
    let key = wow_map::MapKey::new(1, 0);
    // No canonical instance for map 1 yet: admission defers.
    d3b2_register_like_cpp(&mut session, 1, guid);

    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .deferred_creature_respawn_count_like_cpp(key),
        1,
        "the canonical manager holds the deferred registration"
    );
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_creature(1, 0, guid)
            .is_none()
    );

    // `MapManager::CreateMap`: the instance takes the deferred entry into its
    // own respawn queue, which the executor's lifecycle phase drains.
    canonical.lock().unwrap().create_world_map(1, 0);
    let guard = canonical.lock().unwrap();
    assert_eq!(guard.deferred_creature_respawn_count_like_cpp(key), 0);
    let queue = guard
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_respawn_queue_like_cpp();
    assert_eq!(queue.respawn_queue_len(), 1);
    drop(guard);

    // With the instance present, a further deferral joins its queue directly.
    let second = test_creature_guid(97_011);
    let pending = crate::map_manager::pending_respawn_from_world_creature_like_cpp(
        &crate::map_manager::WorldCreature::new(
            second,
            9001,
            Position::new(31.0, 30.0, 0.0, 0.0),
            25,
            1,
            3,
            5,
            20.0,
            0,
            35,
            0,
            0,
        ),
        Instant::now(),
        1,
    );
    canonical
        .lock()
        .unwrap()
        .defer_creature_respawn_like_cpp(key, pending);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(1, 0)
            .unwrap()
            .map()
            .creature_respawn_queue_like_cpp()
            .respawn_queue_len(),
        2
    );
}
