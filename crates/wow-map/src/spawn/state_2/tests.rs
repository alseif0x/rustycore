//! Tagged ownership, legacy ordering and recoverable handoff contracts.
use super::*;
use crate::map_manager::{
    LegacyRespawnTimeAddOutcomeLikeCpp as Saved, pending_respawn_from_world_creature_like_cpp,
};
use crate::{MapKey, map_manager::WorldCreature};
use std::sync::Mutex;
use std::time::Duration;
use wow_core::{Position, guid::HighGuid};
use wow_entities::Creature;

fn pending(id: u64, persistent: bool, due: Instant) -> PendingRespawn {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            1,
            571,
            7,
            42,
            id as i64,
        ));
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(if persistent { id } else { 0 });
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let actor = WorldCreature::from_canonical(creature, data);
    pending_respawn_from_world_creature_like_cpp(&actor, due, 571)
}

fn info(id: u64, seconds: i64) -> RespawnInfoLikeCpp {
    RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: id,
        entry: 42,
        respawn_time: seconds,
        grid_id: 7,
    }
}

fn row(id: u64, seconds: i64) -> PersistedRespawnRowLikeCpp {
    PersistedRespawnRowLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: id,
        respawn_time: seconds,
        map_id: 571,
        instance_id: 7,
    }
}

#[test]
fn saved_only_is_invisible_until_an_explicit_catalog_queue_request() {
    let mut store = RespawnStoreLikeCpp::new();
    assert_eq!(store.save_row(row(1, 20)), Saved::Inserted);
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
    assert!(
        store
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
            .is_none()
    );
    assert_eq!(store.respawn_timer_keys_like_cpp().count(), 0);
    assert_eq!(store.actor_queue_len(), 0);
    assert!(
        store
            .process_due_respawns_like_cpp(
                100,
                |_, _| panic!("saved row is not queued"),
                |_| panic!("saved row is not queued")
            )
            .is_empty()
    );
    assert_eq!(
        store.add_respawn_info_like_cpp(info(1, 20)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );
    assert_eq!(
        store.catalog_timer_keys().collect::<Vec<_>>(),
        vec![(SpawnObjectType::Creature, 1)]
    );
    assert_eq!(store.saved_rows(), vec![row(1, 20)]);
}

#[test]
fn saved_rows_keep_zero_type_and_earlier_equal_filters() {
    let mut store = RespawnStoreLikeCpp::new();
    assert_eq!(store.save_row(row(0, 1)), Saved::RejectedZeroSpawnId);
    let mut unsupported = row(1, 1);
    unsupported.object_type = SpawnObjectType::AreaTrigger;
    assert_eq!(store.save_row(unsupported), Saved::RejectedUnsupportedType);
    assert_eq!(store.save_row(row(1, 20)), Saved::Inserted);
    assert_eq!(
        store.save_row(row(1, 21)),
        Saved::RejectedExistingSoonerOrEqual
    );
    assert_eq!(store.save_row(row(1, 20)), Saved::ReplacedExisting);
    assert_eq!(store.save_row(row(1, 19)), Saved::ReplacedExisting);
    let mut gameobject = row(1, 18);
    gameobject.object_type = SpawnObjectType::GameObject;
    assert_eq!(store.save_row(gameobject), Saved::Inserted);
    assert_eq!(
        store.saved_row(SpawnObjectType::Creature, 1),
        Some(&row(1, 19))
    );
    assert_eq!(
        store.saved_row(SpawnObjectType::GameObject, 1),
        Some(&gameobject)
    );
    assert_eq!(store.respawn_timer_keys_like_cpp().count(), 0);
}

#[test]
fn actor_and_catalog_have_one_executor_despite_an_earlier_actor_unix_head() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(info(1, 0));
    store
        .queue_actor(pending(1, true, now + Duration::from_secs(60)))
        .unwrap();
    store.add_respawn_info_like_cpp(info(2, 20));
    let mut seen = Vec::new();
    let actions = store.process_due_respawns_like_cpp(
        20,
        |_, _| None,
        |i| {
            seen.push(i.spawn_id);
            CheckRespawnOutcomeLikeCpp::Allowed
        },
    );
    assert_eq!(seen, vec![2]);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
    assert!(
        store
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
            .is_some()
    );
    assert!(store.drain_ready_actors(now).is_empty());
    assert_eq!(store.actor_queue_len(), 1);
}

#[test]
fn future_actor_info_does_not_block_an_earlier_due_catalog_entry() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store.queue_actor(pending(1, true, now)).unwrap();
    store.add_respawn_info_like_cpp(info(1, 500));
    store.add_respawn_info_like_cpp(info(2, 30));
    assert_eq!(
        store
            .process_due_respawns_like_cpp(30, |_, _| None, |_| CheckRespawnOutcomeLikeCpp::Allowed)
            .len(),
        1
    );
    assert_eq!(store.drain_ready_actors(now).len(), 1);
}

#[test]
fn add_info_on_actor_preserves_payload_address_deadline_and_ordinal() {
    let now = Instant::now();
    let deadline = now + Duration::from_nanos(999);
    let mut store = RespawnStoreLikeCpp::new();
    store.add_respawn_info_like_cpp(info(1, 20));
    let mut incoming = pending(1, true, deadline);
    incoming.script_name = "captured-runtime-script".into();
    store.queue_actor(incoming).unwrap();
    store.queue_actor(pending(2, true, now)).unwrap();
    let key = RespawnKey::Persistent(SpawnObjectType::Creature, 1);
    let address = match store.slots.get(&key).unwrap() {
        RespawnSlot::QueuedActor { payload, .. } => &**payload as *const PendingRespawn,
        _ => unreachable!(),
    };
    assert_eq!(
        store.add_respawn_info_like_cpp(info(1, 21)),
        AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual
    );
    assert_eq!(
        store.add_respawn_info_like_cpp(info(1, 19)),
        AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    assert_eq!(
        store.actor_order,
        vec![key, RespawnKey::Persistent(SpawnObjectType::Creature, 2)]
    );
    match store.slots.get(&key).unwrap() {
        RespawnSlot::QueuedActor { payload, .. } => {
            assert_eq!(&**payload as *const PendingRespawn, address);
            assert_eq!(payload.respawn_at, deadline);
            assert_eq!(payload.script_name, "captured-runtime-script");
        }
        _ => panic!("INFO must retain Actor ownership"),
    }
    assert_eq!(
        store
            .drain_ready_actors(now)
            .iter()
            .map(|p| p.spawn_id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(
        store
            .drain_ready_actors(deadline - Duration::from_nanos(1))
            .is_empty()
    );
    assert_eq!(store.drain_ready_actors(deadline)[0].spawn_id, 1);
}

#[test]
fn actor_replacement_remove_append_and_later_rejection_return_intact_payload() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store
        .queue_actor(pending(1, true, now + Duration::from_secs(2)))
        .unwrap();
    store.queue_actor(pending(2, true, now)).unwrap();
    let mut later = pending(1, true, now + Duration::from_secs(3));
    later.script_name = "returned".into();
    let rejected = store.queue_actor(later).unwrap_err();
    assert_eq!(rejected.script_name, "returned");
    assert_eq!(rejected.respawn_at, now + Duration::from_secs(3));
    store.queue_actor(pending(1, true, now)).unwrap();
    assert_eq!(
        store
            .drain_ready_actors(now)
            .iter()
            .map(|p| p.spawn_id)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
}

#[test]
fn equal_deadline_replacement_reappends_without_sorting_other_ready_entries() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store.queue_actor(pending(1, true, now)).unwrap();
    store
        .queue_actor(pending(2, true, now - Duration::from_secs(1)))
        .unwrap();
    store.queue_actor(pending(1, true, now)).unwrap();
    assert_eq!(
        store
            .drain_ready_actors(now)
            .iter()
            .map(|p| p.spawn_id)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
}

#[test]
fn persistent_spawn_and_transient_guid_low_never_alias_or_create_fake_info() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store.save_row(row(42, 1));
    store.queue_actor(pending(42, true, now)).unwrap();
    store.queue_actor(pending(42, false, now)).unwrap();
    assert_eq!(store.actor_queue_len(), 2);
    assert_eq!(store.slots.len(), 2);
    assert_eq!(store.respawn_timer_keys_like_cpp().count(), 0);
    let ready = store.drain_ready_actors(now);
    assert_eq!(
        ready.iter().map(|p| p.persistent_spawn).collect::<Vec<_>>(),
        vec![true, false]
    );
    assert_eq!(store.saved_rows(), vec![row(42, 1)]);
}

#[test]
fn info_removal_and_unload_preserve_actor_but_cancel_owned_removes_everything() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store.save_row(row(1, 20));
    store.queue_actor(pending(1, true, now)).unwrap();
    store.add_respawn_info_like_cpp(info(1, 20));
    assert!(
        store
            .remove_respawn_time_like_cpp(SpawnObjectType::Creature, 1)
            .is_some()
    );
    assert_eq!(store.actor_queue_len(), 1);
    assert_eq!(store.saved_rows(), vec![row(1, 20)]);
    store.add_respawn_info_like_cpp(info(1, 20));
    store.unload_all_respawn_infos_like_cpp();
    assert_eq!(store.actor_queue_len(), 1);
    let removed = store
        .cancel_owned(RespawnKey::Persistent(SpawnObjectType::Creature, 1))
        .unwrap();
    assert_eq!(removed.into_actor().unwrap().spawn_id, 1);
    assert!(store.saved_rows().is_empty());
    assert!(store.drain_ready_actors(now).is_empty());
}

#[test]
fn transfer_rejects_wrong_key_epoch_and_actor_collision_without_partial_mutation() {
    let now = Instant::now();
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    let key = MapKey::new(571, 7);
    let mut source = RespawnStoreLikeCpp::new();
    source.queue_actor(pending(1, true, now)).unwrap();
    source.queue_actor(pending(2, true, now)).unwrap();
    let mut destination = RespawnStoreLikeCpp::new();
    destination.queue_actor(pending(2, true, now)).unwrap();
    let transfer = source.take_transfer(key, 9, &guard);
    let (error, transfer) = destination
        .accept_transfer(transfer, MapKey::new(530, 7), 9)
        .unwrap_err();
    assert_eq!(error, RespawnTransferError::WrongMap);
    let (error, transfer) = destination.accept_transfer(transfer, key, 10).unwrap_err();
    assert_eq!(error, RespawnTransferError::StaleIncarnation);
    let (error, transfer) = destination.accept_transfer(transfer, key, 9).unwrap_err();
    assert_eq!(
        error,
        RespawnTransferError::OccupiedActor {
            key: RespawnKey::Persistent(SpawnObjectType::Creature, 2)
        }
    );
    assert_eq!(destination.actor_queue_len(), 1);
    assert!(
        !destination
            .slots
            .contains_key(&RespawnKey::Persistent(SpawnObjectType::Creature, 1))
    );
    source.restore_transfer(transfer).unwrap();
    assert_eq!(
        source
            .drain_ready_actors(now)
            .iter()
            .map(|p| p.spawn_id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn transfer_moves_the_same_box_and_preserves_exact_clock_and_catalog_collision_filter() {
    let now = Instant::now();
    let due = now + Duration::from_nanos(123);
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    let key = MapKey::new(571, 7);
    let mut source = RespawnStoreLikeCpp::new();
    source.save_row(row(1, 40));
    source.queue_actor(pending(1, true, due)).unwrap();
    source.add_respawn_info_like_cpp(info(1, 40));
    let slot_key = RespawnKey::Persistent(SpawnObjectType::Creature, 1);
    let address = match source.slots.get(&slot_key).unwrap() {
        RespawnSlot::QueuedActor { payload, .. } => &**payload as *const PendingRespawn,
        _ => unreachable!(),
    };
    let mut destination = RespawnStoreLikeCpp::new();
    destination.add_respawn_info_like_cpp(info(1, 30));
    destination
        .accept_transfer(source.take_transfer(key, 9, &guard), key, 9)
        .unwrap();
    assert_eq!(source.actor_queue_len(), 0);
    assert_eq!(
        destination.get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        30
    );
    assert_eq!(destination.catalog_timer_keys().count(), 0);
    match destination.slots.get(&slot_key).unwrap() {
        RespawnSlot::QueuedActor { payload, .. } => {
            assert_eq!(&**payload as *const PendingRespawn, address);
            assert_eq!(payload.respawn_at, due);
        }
        _ => unreachable!(),
    }
    assert!(destination.drain_ready_actors(now).is_empty());
    assert_eq!(destination.drain_ready_actors(due).len(), 1);
}

#[test]
fn transfer_wrong_payload_map_returns_all_entries_and_drop_never_executes_them() {
    let now = Instant::now();
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    let key = MapKey::new(571, 7);
    let mut source = RespawnStoreLikeCpp::new();
    source.save_row(row(1, 1));
    let mut other_map = pending(2, false, now);
    other_map.map_id = 530;
    source.queue_actor(other_map).unwrap();
    let mut destination = RespawnStoreLikeCpp::new();
    let (error, transfer) = destination
        .accept_transfer(source.take_transfer(key, 9, &guard), key, 9)
        .unwrap_err();
    assert_eq!(error, RespawnTransferError::WrongMap);
    assert!(destination.slots.is_empty());
    source.restore_transfer(transfer).unwrap();
    assert_eq!(source.saved_rows(), vec![row(1, 1)]);
    let transfer = source.take_transfer(key, 9, &guard);
    drop(transfer); // Explicit terminal ownership disposition; no implicit scheduler.
    assert!(source.slots.is_empty());
    assert!(source.drain_ready_actors(now).is_empty());
    assert!(destination.slots.is_empty());
}

#[test]
fn catalog_reschedule_changes_unix_index_without_rescheduling_the_actor_lane() {
    let now = Instant::now();
    let mut store = RespawnStoreLikeCpp::new();
    store
        .queue_actor(pending(1, true, now + Duration::from_secs(60)))
        .unwrap();
    store.add_respawn_info_like_cpp(info(1, 0));
    store.add_respawn_info_like_cpp(info(2, 20));
    let actions = store.process_due_respawns_like_cpp(
        20,
        |_, _| None,
        |info| {
            assert_eq!(info.spawn_id, 2);
            info.respawn_time = 30;
            CheckRespawnOutcomeLikeCpp::Blocked
        },
    );
    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::RescheduleAndSave { info: info(2, 30) }]
    );
    assert_eq!(
        store.get_respawn_time_like_cpp(SpawnObjectType::Creature, 2),
        30
    );
    assert_eq!(store.actor_queue_len(), 1);
    assert!(
        store
            .process_due_respawns_like_cpp(
                29,
                |_, _| panic!("future Catalog"),
                |_| panic!("future Catalog")
            )
            .is_empty()
    );
    assert_eq!(
        store
            .process_due_respawns_like_cpp(30, |_, _| None, |_| CheckRespawnOutcomeLikeCpp::Allowed)
            .len(),
        1
    );
    assert!(store.drain_ready_actors(now).is_empty());
}

#[test]
fn saved_only_transfer_preserves_existing_explicit_catalog_request() {
    let fence = Mutex::new(());
    let guard = fence.lock().unwrap();
    let key = MapKey::new(571, 7);
    let mut source = RespawnStoreLikeCpp::new();
    source.save_row(row(1, 40));
    let mut destination = RespawnStoreLikeCpp::new();
    destination.add_respawn_info_like_cpp(info(1, 30));
    destination
        .accept_transfer(source.take_transfer(key, 9, &guard), key, 9)
        .unwrap();
    assert_eq!(
        destination.catalog_timer_keys().collect::<Vec<_>>(),
        vec![(SpawnObjectType::Creature, 1)]
    );
    assert_eq!(
        destination.get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        30
    );
    assert_eq!(destination.saved_rows(), vec![row(1, 40)]);
}
