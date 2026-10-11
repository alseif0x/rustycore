//! F6-7 R1b regressions: one incarnation per creature, through the respawn
//! rail.
//!
//! R1a admitted one loot authority per creature incarnation at registration;
//! R1b sealed the construction, publication and rebinding paths. Until #1263
//! F6-8E a creature could coexist as the canonical incarnation and a legacy
//! representation, and most of these tests pinned the second representation
//! adopting the admitted incarnation's authority and health timeline. F6-8E
//! deleted the legacy representation; what remains is the canonical respawn
//! rail's own contract:
//!
//! * a respawn allocates exactly one incarnation, with one loot allocation and
//!   one health timeline;
//! * R7b-1: a canonical manager without a map instance for the key provides no
//!   owner, so a ready respawn is **deferred** rather than published, and is
//!   published once the instance legitimately exists;
//! * an in-place respawn retains the incarnation's allocation and invalidates
//!   leases of the previous loot lifetime.
//!
//! These tests drive the production lifecycle phase body
//! (`run_creature_lifecycle_phase_on_manager_like_cpp`) rather than a double.

use super::*;

fn test_position_like_cpp() -> Position {
    Position::new(10.0, 10.0, 0.0, 0.0)
}

fn corpse_loot_like_cpp(guid: ObjectGuid, coins: u32, player: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::create_world_object(
            HighGuid::LootObject,
            0,
            0,
            0,
            0,
            0,
            i64::from(coins),
        ),
        coins,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player],
        items: Vec::new(),
        looted_by_player: false,
    }
}

/// The canonical creature for `guid`, if any.
fn canonical_creature_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> Option<wow_entities::Creature> {
    canonical
        .lock()
        .ok()?
        .find_map(0, 0)?
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
}

/// Queue one ready respawn for `guid` and run the production lifecycle phase
/// once.
fn run_respawn_tick_like_cpp(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    hp: u32,
    now: Instant,
) -> crate::session::CreatureLifecycleTickOutcomeLikeCpp {
    let queued = crate::creature_projection::new_runtime_creature_like_cpp(
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
    let pending = crate::creature_respawn::pending_respawn_from_creature_like_cpp(&queued, now, 0);
    {
        let mut guard = manager
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // As the production registration does: a map that does not exist yet
        // holds the respawn on the canonical side until `CreateMap`.
        if guard.find_map(0, 0).is_some() {
            guard.push_respawn(0, 0, pending);
        } else {
            guard.defer_creature_respawn_like_cpp(wow_map::MapKey::new(0, 0), pending);
        }
    }
    run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        manager,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now + Duration::from_secs(1),
    )
}

#[test]
fn lifecycle_respawn_fresh_incarnation_allocates_one_incarnation_like_cpp() {
    let canonical = shared_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let now = Instant::now();
    let guid = test_creature_guid(91_301);
    let player = ObjectGuid::create_player(1, 91_301);

    let outcome = run_respawn_tick_like_cpp(&canonical, guid, 105, now - Duration::from_secs(1));
    assert_eq!(outcome.respawns_processed, 1);
    assert_eq!(outcome.canonical_inserts, 1);

    let canonical_creature = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    assert_eq!(canonical_creature.unit().data().health, 105);

    let canonical_authority = canonical_creature.loot_authority_like_cpp().clone();
    canonical_authority.initialize_shared_like_cpp(corpse_loot_like_cpp(guid, 17, player));
    assert_eq!(
        canonical_creature_like_cpp(&canonical, guid)
            .unwrap()
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .expect("the fresh incarnation owns one pool")
            .loot
            .coins,
        17
    );
}

#[test]
fn lifecycle_respawn_without_canonical_incarnation_defers_publication_like_cpp() {
    // F6-7 R7b-1 contract. A canonical manager without a canonical map instance
    // for this key provides no owner, so the ready respawn must not be
    // published; the built candidate is held until legitimate admission.
    let canonical = shared_map_manager();
    let now = Instant::now();
    let guid = test_creature_guid(91_302);

    let outcome = run_respawn_tick_like_cpp(&canonical, guid, 105, now - Duration::from_secs(1));
    assert_eq!(
        outcome.respawns_processed, 0,
        "a respawn with no canonical owner is not published"
    );
    assert_eq!(outcome.canonical_inserts, 0);
    assert!(
        canonical_creature_like_cpp(&canonical, guid).is_none(),
        "no canonical incarnation was admitted"
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .deferred_creature_respawn_count_like_cpp(wow_map::MapKey::new(0, 0)),
        1,
        "the built respawn candidate waits for legitimate admission"
    );

    // Eventual admission: once the canonical instance legitimately exists, the
    // next lifecycle phase publishes the deferred respawn.
    canonical.lock().unwrap().create_world_map(0, 0);
    let second = run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        &canonical,
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now + Duration::from_secs(2),
    );
    assert_eq!(
        canonical.lock().unwrap().respawn_queue_len(0, 0),
        0,
        "the deferred respawn is no longer waiting"
    );
    assert_eq!(
        second.respawns_processed, 1,
        "the deferred respawn is published once it can be admitted"
    );
    let canonical_creature =
        canonical_creature_like_cpp(&canonical, guid).expect("the incarnation is admitted");
    assert_eq!(canonical_creature.unit().data().health, 105);
    assert_eq!(
        second.refresh_map_keys,
        vec![(0, 0)],
        "the map key is signalled so sessions recompute visibility"
    );
}

#[tokio::test]
async fn in_place_respawn_retains_allocation_and_invalidates_old_leases_like_cpp() {
    let canonical = shared_map_manager();
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    canonical.lock().unwrap().create_world_map(0, 0);
    let guid = test_creature_guid(91_307);
    let player = ObjectGuid::create_player(1, 91_307);
    register_test_creature(&mut session, canonical.clone(), guid, 25);
    let canonical_authority = canonical_creature_like_cpp(&canonical, guid)
        .expect("fresh admission")
        .loot_authority_like_cpp()
        .clone();
    canonical_authority.initialize_shared_like_cpp(corpse_loot_like_cpp(guid, 60, player));
    let lease = canonical_authority
        .reserve_money_like_cpp(player)
        .await
        .expect("the admitted pool is claimable");
    assert!(lease.shares_authority_like_cpp(&canonical_authority));

    // The object respawns in place: C++ `ClearLoot` retires the lifetime and the
    // next generation installs onto the same object-owned allocation.
    {
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .expect("canonical creature");
        creature.clear_loot_like_cpp();
        assert_ne!(
            creature.replace_loot_authority_like_cpp(
                Some(corpse_loot_like_cpp(guid, 12, player)),
                HashMap::new(),
            ),
            0,
            "the in-place respawn installs a new lifetime"
        );
    }

    assert!(
        canonical_authority.shares_storage_like_cpp(
            canonical_creature_like_cpp(&canonical, guid)
                .unwrap()
                .loot_authority_like_cpp()
        ),
        "an in-place respawn retains the incarnation's allocation"
    );
    assert_eq!(
        lease.commit_with_snapshot_like_cpp().unwrap_err(),
        wow_loot::LootClaimCommitError::StaleGeneration,
        "a lease taken before the in-place respawn cannot commit into the new lifetime"
    );
    assert_eq!(
        canonical_authority
            .shared_snapshot_like_cpp()
            .expect("the new lifetime is openable")
            .loot
            .coins,
        12
    );
}
