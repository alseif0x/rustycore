//! Real APP lock/fence composition; the producer is not started by any case.

use super::*;
use std::sync::{Arc, Mutex, RwLock};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{Creature, MapObjectRecord, OwnedLootAuthority};
use wow_map::map_manager::WorldCreature;

fn owners() -> (SharedMapManager, SharedCanonicalMapManager, ObjectGuid, u64) {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, 861);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(8610);
    let canonical_snapshot = creature.clone(); // Initial existing bridge only.
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let mut actor = WorldCreature::from_canonical(creature, data);
    actor.create_data.npc_flags = 0x1234;
    actor.creature.unit_mut().subsystems_mut().spells.set_cooldown(133, 500, 1500);
    let mut legacy = crate::map_manager::MapManager::new();
    assert!(legacy.add_creature(571, 7, 0, 0, actor));
    let mut canonical = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    canonical.create_world_map(571, 7).map_mut()
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(canonical_snapshot).unwrap()).unwrap();
    let incarnation = canonical.map_incarnation_like_cpp(MapKey::new(571, 7)).unwrap();
    (Arc::new(RwLock::new(legacy)), Arc::new(Mutex::new(canonical)), guid, incarnation)
}

#[test]
fn associated_operation_moves_whole_actor_under_existing_fence_and_both_guards() {
    let (legacy, canonical, guid, incarnation) = owners();
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    assert_eq!(WorldSession::transfer_legacy_creature_ownership(&legacy, &canonical, MapKey::new(571, 7), incarnation, &guard).unwrap().source_inner_winners, 1);
    assert!(legacy.read().unwrap().get_map(571, 7).unwrap().grids.values().all(|grid| grid.creatures.is_empty()));
    let canonical = canonical.lock().unwrap();
    let creature = canonical.find_map(571, 7).unwrap().map().get_typed_creature(guid).unwrap();
    assert_eq!(creature.current_health(), 75);
    assert_eq!(creature.unit().subsystems().spells.remaining_cooldown_ms(133, 0, 500), 1500);
    assert_eq!(canonical.tick_coordination_like_cpp(), wow_map::MapTickCoordinationStateLikeCpp::Idle);
}

#[test]
fn stale_busy_and_missing_source_rejections_do_not_extract_or_create_owners() {
    let (legacy, canonical, guid, incarnation) = owners();
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    assert_eq!(WorldSession::transfer_legacy_creature_ownership(&legacy, &canonical, MapKey::new(571, 7), incarnation + 1, &guard), Err(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::StaleIncarnation)));
    let empty = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    assert_eq!(WorldSession::transfer_legacy_creature_ownership(&empty, &canonical, MapKey::new(571, 7), incarnation, &guard), Err(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::MissingLegacyMap)));
    assert!(empty.read().unwrap().active_map_keys().is_empty());
    let plan = canonical.lock().unwrap().begin_tick_like_cpp(200).into_started().unwrap();
    assert_eq!(WorldSession::transfer_legacy_creature_ownership(&legacy, &canonical, MapKey::new(571, 7), incarnation, &guard), Err(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::MapBusy)));
    assert!(legacy.read().unwrap().get_creature(571, 7, 0, 0, guid).is_some());
    drop(plan);
}

#[test]
fn divergent_loot_returns_explicit_admission_failure_without_reconciliation() {
    let (legacy, canonical, guid, incarnation) = owners();
    let authority = OwnedLootAuthority::new();
    legacy.write().unwrap().get_creature_mut(571, 7, 0, 0, guid).unwrap()
        .creature.adopt_loot_authority_for_snapshot_like_cpp(authority.clone());
    let stamp = authority.stamp_like_cpp();
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    assert_eq!(WorldSession::transfer_legacy_creature_ownership(&legacy, &canonical, MapKey::new(571, 7), incarnation, &guard), Err(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::LootAuthorityMismatch { guid })));
    assert_eq!(authority.stamp_like_cpp(), stamp);
    assert!(legacy.read().unwrap().get_creature(571, 7, 0, 0, guid).is_some());
    assert_eq!(canonical.lock().unwrap().find_map(571, 7).unwrap().map().map_object_count(), 1);
}

#[test]
fn poisoned_manager_guards_reject_without_taking_either_owned_value() {
    for canonical_poison in [false, true] {
        let (legacy, canonical, guid, incarnation) = owners();
        let _poison = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if canonical_poison {
                let _guard = canonical.lock().unwrap();
                panic!("fixture canonical poison");
            } else {
                let _guard = legacy.write().unwrap();
                panic!("fixture legacy poison");
            }
        }));
        let fence = Mutex::new(());
        let guard = fence.lock().unwrap();
        let expected = if canonical_poison {
            CreatureOwnershipTransferError::CanonicalUnavailable
        } else {
            CreatureOwnershipTransferError::LegacyUnavailable
        };
        assert_eq!(WorldSession::transfer_legacy_creature_ownership(&legacy, &canonical, MapKey::new(571, 7), incarnation, &guard), Err(expected));
        {
            let source = match legacy.read() { Ok(guard) => guard, Err(poison) => poison.into_inner() };
            assert!(source.get_creature(571, 7, 0, 0, guid).is_some());
        }
        let destination = match canonical.lock() { Ok(guard) => guard, Err(poison) => poison.into_inner() };
        assert_eq!(destination.find_map(571, 7).unwrap().map().map_object_count(), 1);
    }
}
