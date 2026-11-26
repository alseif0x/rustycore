//! Chest money restock and durable source transition scenarios.

use super::*;

fn marked_chest(counter: i64, spawn_id: u64, consumable: bool, state: LootState) -> GameObject {
    let mut gameobject = game_object_with_counter(counter, 571, 7, false);
    gameobject.set_spawn_id(spawn_id);
    assert!(gameobject.bind_chest_money_generation(2, 0, |_| true));
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_restock_time_secs: 45,
        chest_consumable: consumable,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_loot_state(state, None);
    let player = ObjectGuid::create_player(1, counter);
    assert!(
        gameobject
            .initialize_shared_loot_authority_like_cpp(CreatureLoot {
                loot_guid: gameobject.world().guid(),
                coins: 41,
                unlooted_count: 2,
                loot_type: 1,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player],
                items: Vec::new(),
                looted_by_player: false,
            })
            .installed()
    );
    gameobject
}

fn add_chest(map: &mut Map<RecordingTerrain, RecordingLifecycle>, gameobject: GameObject) {
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
}

#[test]
fn fully_looted_non_consumable_chest_waits_for_durable_arm_and_preserves_origin_deadline_like_cpp()
{
    let mut map = test_map();
    let mut gameobject = marked_chest(4610101, 4610101, false, LootState::JustDeactivated);
    let guid = gameobject.world().guid();
    // A prior activation timer is stale once Map sees JustDeactivated. This
    // branch starts a fresh interval from the fully-looted event time.
    gameobject.set_restock_time_like_cpp(900);
    add_chest(&mut map, gameobject);

    let waiting = map.update_game_object_like_cpp(guid, 1, 1_000);
    assert_eq!(waiting.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(!waiting.loot_cleared);
    assert_eq!(
        map.get_typed_game_object(guid).unwrap().loot_state(),
        LootState::JustDeactivated
    );
    assert_eq!(map.get_typed_game_object(guid).unwrap().restock_time(), 900);
    assert!(
        map.get_typed_game_object(guid)
            .unwrap()
            .shared_loot_like_cpp()
            .is_some()
    );
    let event = map.take_chest_money_restocks().pop().unwrap();
    assert_eq!(
        (event.spawn_id, event.generation, event.deadline),
        (4610101, 0, 1_045)
    );
    assert!(!event.persist_respawn);
    assert!(map.take_chest_money_restocks().is_empty());

    let refused = map.update_game_object_like_cpp(guid, 1, 1_001);
    assert!(!refused.loot_cleared);
    assert!(map.has_pending_chest_money_resets_like_cpp());
    assert!(event.acknowledged.set(true).is_ok());

    let applied = map.update_game_object_like_cpp(guid, 1, 1_002);
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert!(applied.loot_cleared);
    assert!(applied.non_consumed_restock_armed);
    assert_eq!(canonical.restock_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert!(!map.has_pending_chest_money_resets_like_cpp());
    let canonical = map.get_typed_game_object_mut(guid).unwrap();
    assert!(!canonical.bind_chest_money_generation(2, 0, |_| true));
    assert!(canonical.bind_chest_money_generation(2, 1, |_| true));
}

#[test]
fn non_consumable_partial_timer_expires_only_for_activated_live_loot_and_waits_for_ack_like_cpp() {
    let mut map = test_map();
    let mut gameobject = marked_chest(4610111, 4610111, false, LootState::Activated);
    let guid = gameobject.world().guid();
    gameobject.set_restock_time_like_cpp(1_045);
    add_chest(&mut map, gameobject);

    let before_deadline = map.update_game_object_like_cpp(guid, 1, 1_044);
    assert!(!before_deadline.loot_cleared);
    assert!(!map.has_pending_chest_money_resets_like_cpp());
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert_eq!(canonical.restock_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::Activated);
    assert!(!canonical.loot_authority_like_cpp().is_retired_like_cpp());
    assert_eq!(
        canonical.loot_authority_like_cpp().lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active
    );
    let player = ObjectGuid::create_player(1, 4610111);
    let payout = poll_immediately_ready(
        canonical
            .loot_authority_like_cpp()
            .reserve_money_like_cpp(player),
    )
    .expect("the real chest money pool grants its eligible player");
    let mut persistence = payout
        .begin_persistence_guard_like_cpp()
        .expect("the pre-deadline payout enters its durable phase");

    let at_deadline = map.update_game_object_like_cpp(guid, 1, 1_045);
    assert!(!at_deadline.loot_cleared);
    assert!(map.has_pending_chest_money_resets_like_cpp());
    assert_eq!(map.acknowledged_chest_money_guids_like_cpp().count(), 0);
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert_eq!(canonical.restock_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::Activated);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert!(canonical.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(persistence.commit_like_cpp().unwrap());
    let event = map.take_chest_money_restocks().pop().unwrap();
    assert_eq!(event.deadline, 1_045);
    assert!(!event.persist_respawn);

    let still_waiting = map.update_game_object_like_cpp(guid, 1, 1_046);
    assert!(!still_waiting.loot_cleared);
    assert!(map.chest_money_reset_pending_like_cpp(guid));
    assert!(event.acknowledged.set(true).is_ok());

    let applied = map.update_game_object_like_cpp(guid, 1, 1_047);
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert!(applied.loot_cleared);
    assert!(applied.non_consumed_set_ready);
    assert_eq!(canonical.restock_time(), 0);
    assert_eq!(canonical.loot_state(), LootState::Ready);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert!(!map.has_pending_chest_money_resets_like_cpp());
}

#[test]
fn non_consumable_partial_timer_refusal_keeps_pool_and_map_resident_like_cpp() {
    let mut map = test_map();
    let mut gameobject = marked_chest(4610121, 4610121, false, LootState::Activated);
    let guid = gameobject.world().guid();
    gameobject.set_restock_time_like_cpp(1_000);
    add_chest(&mut map, gameobject);

    let due = map.update_game_object_like_cpp(guid, 1, 1_000);
    assert!(!due.loot_cleared);
    let event = map.take_chest_money_restocks().pop().unwrap();
    assert!(event.acknowledged.set(false).is_ok());
    let refused = map.update_game_object_like_cpp(guid, 1, 1_001);
    assert!(!refused.loot_cleared);
    assert!(map.has_pending_chest_money_resets_like_cpp());
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert_eq!(canonical.restock_time(), 1_000);
    assert_eq!(canonical.loot_state(), LootState::Activated);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert!(canonical.loot_authority_like_cpp().is_retired_like_cpp());
}

#[test]
fn stale_not_ready_timer_does_not_take_activated_partial_expiry_branch_like_cpp() {
    let mut map = test_map();
    let mut gameobject = marked_chest(4610131, 4610131, false, LootState::NotReady);
    let guid = gameobject.world().guid();
    gameobject.set_restock_time_like_cpp(1_000);
    add_chest(&mut map, gameobject);

    let outcome = map.update_game_object_like_cpp(guid, 1, 1_001);
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert!(!outcome.loot_cleared);
    assert!(!map.has_pending_chest_money_resets_like_cpp());
    assert_eq!(canonical.restock_time(), 1_000);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert!(canonical.shared_loot_like_cpp().is_some());
}

#[test]
fn removed_chest_arm_keeps_one_submission_without_pinning_same_guid_replacement_like_cpp() {
    let mut map = test_map();
    let mut old = marked_chest(4610135, 4610135, false, LootState::Activated);
    let guid = old.world().guid();
    old.set_restock_time_like_cpp(1_000);
    add_chest(&mut map, old);

    let due = map.update_game_object_like_cpp(guid, 1, 1_000);
    assert!(!due.loot_cleared);
    let old_authority = map
        .get_typed_game_object(guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    assert!(map.has_pending_chest_money_resets_like_cpp());

    assert!(map.remove_map_object(guid).is_some());
    assert_eq!(
        map.update_game_object_like_cpp(guid, 1, 1_001).status,
        GameObjectUpdateStatusLikeCpp::MissingGameObject
    );
    assert!(!map.has_pending_chest_money_resets_like_cpp());
    // The retired generation's already-authorized durable command remains
    // available exactly once even after its owning object has been removed.
    let old_events = map.take_chest_money_restocks();
    assert_eq!(old_events.len(), 1);
    let old_event = old_events.into_iter().next().unwrap();
    assert!(!map.has_pending_chest_money_resets_like_cpp());
    assert!(map.take_chest_money_restocks().is_empty());

    // The replacement deliberately reuses the old GUID, spawn id, and local
    // lifecycle revision. Its canonical loot authority is a new lifetime.
    let mut replacement = marked_chest(4610135, 4610135, false, LootState::Activated);
    replacement.set_restock_time_like_cpp(1_001);
    add_chest(&mut map, replacement);
    let replacement_authority = map
        .get_typed_game_object(guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    assert!(!old_authority.shares_storage_like_cpp(&replacement_authority));
    assert!(!map.chest_money_reset_pending_like_cpp(guid));
    assert_eq!(map.acknowledged_chest_money_guids_like_cpp().count(), 0);

    let replacement_due = map.update_game_object_like_cpp(guid, 1, 1_001);
    assert!(!replacement_due.loot_cleared);
    let replacement_events = map.take_chest_money_restocks();
    assert_eq!(replacement_events.len(), 1);
    let replacement_event = replacement_events.into_iter().next().unwrap();
    assert_eq!(replacement_event.spawn_id, old_event.spawn_id);
    assert_eq!(
        replacement_event.lifecycle_revision,
        old_event.lifecycle_revision
    );
    assert_eq!(replacement_event.generation, old_event.generation);
    assert!(old_event.acknowledged.set(true).is_ok());
    assert!(replacement_event.acknowledged.get().is_none());
    assert_eq!(map.acknowledged_chest_money_guids_like_cpp().count(), 0);

    let old_ack_did_not_apply = map.update_game_object_like_cpp(guid, 1, 1_002);
    assert!(!old_ack_did_not_apply.loot_cleared);
    assert!(map.chest_money_reset_pending_like_cpp(guid));
    assert!(replacement_event.acknowledged.set(true).is_ok());
    assert_eq!(map.acknowledged_chest_money_guids_like_cpp().count(), 1);
    let replacement_applied = map.update_game_object_like_cpp(guid, 1, 1_003);
    assert!(replacement_applied.loot_cleared);
    assert!(!map.has_pending_chest_money_resets_like_cpp());
}

#[test]
fn consumable_chest_persists_respawn_with_source_arm_before_clearloot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = marked_chest(4610141, 4610141, true, LootState::JustDeactivated);
    let guid = gameobject.world().guid();
    gameobject.world_mut().object_mut().set_entry(190_014);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_spawned_by_default(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_delay_time(60);
    add_chest(&mut map, gameobject);

    let waiting = map.update_game_object_like_cpp(guid, 1, 1_000);
    assert!(!waiting.loot_cleared);
    let event = map.take_chest_money_restocks().pop().unwrap();
    assert_eq!(event.deadline, 1_060);
    assert!(event.persist_respawn);
    assert!(
        map.get_typed_game_object(guid)
            .unwrap()
            .shared_loot_like_cpp()
            .is_some()
    );

    assert!(event.acknowledged.set(true).is_ok());
    let applied = map.update_game_object_like_cpp(guid, 1, 1_001);
    let canonical = map.get_typed_game_object(guid).unwrap();
    assert!(applied.loot_cleared);
    assert_eq!(applied.generic_respawn_scheduled_time, Some(1_060));
    assert!(applied.generic_respawn_compatibility_db_only_represented);
    assert!(applied.generic_visibility_on_destroy_represented);
    assert_eq!(canonical.respawn_time(), 1_060);
    assert!(canonical.shared_loot_like_cpp().is_none());
    assert!(!map.has_pending_chest_money_resets_like_cpp());
}
