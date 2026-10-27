//! Native QuestLog fixture ownership and Map publication helpers.

use super::*;

pub(super) fn canonical_player_quest_status_like_cpp(
    session: &WorldSession,
    quest_id: u32,
) -> Option<PlayerQuestStatus> {
    session
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|quests| quests.status_like_cpp(quest_id).cloned())
}

pub(super) fn canonical_player_has_rewarded_quest_like_cpp(
    session: &WorldSession,
    quest_id: u32,
) -> bool {
    session
        .player_quest_gameplay_snapshot_like_cpp()
        .is_some_and(|quests| quests.rewarded_quest_ids_like_cpp().contains(&quest_id))
}

fn ensure_native_quest_log_player_owner_like_cpp(
    session: &mut WorldSession,
    map_id: u16,
) {
    let guid = session
        .player_guid()
        .unwrap_or_else(|| wow_core::ObjectGuid::create_player(1, 42));
    session.set_player_guid(Some(guid));

    let manager = match session.core.canonical_map_manager.as_ref().cloned() {
        Some(manager) => manager,
        None => {
            let manager = std::sync::Arc::new(std::sync::Mutex::new(
                wow_map::MapManager::default(),
            ));
            session.set_canonical_map_manager(std::sync::Arc::clone(&manager));
            manager
        }
    };
    let map_id = u32::from(map_id);
    let has_player = manager
        .lock()
        .expect("quest fixture Map manager lock")
        .find_map(map_id, 0)
        .is_some_and(|map| map.map().get_typed_player(guid).is_some());
    if !has_player {
        if crate::session::hub_ref(session)
            .player_name_like_cpp()
            .is_none()
        {
            session.set_loaded_player_name_like_cpp("QuestLogFixture".to_owned());
        }
        let player = session
            .initial_player_fixture_like_cpp()
            .expect("quest fixture Player identity and position");
        manager
            .lock()
            .expect("quest fixture Map manager lock")
            .create_world_map(map_id, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
}

pub(super) fn install_native_quest_log_player_like_cpp(session: &mut WorldSession) {
    if session
        .core
        .quest_objective_access_like_cpp()
        .owner_handle_absent_like_cpp()
    {
        let existing = session
            .player_quest_gameplay_snapshot_like_cpp()
            .expect("unowned quest fixture state");
        let fixture_pending_share = session
            .represented_pending_quest_sharing_like_cpp()
            .map(|pending| (pending.sender_guid, pending.quest_id));
        let fixture_inventory = session
            .inventory
            .resolved_player_inventory_runtime_like_cpp(crate::session::hub_ref(session))
            .expect("unowned inventory fixture state");
        let fixture_currencies = session
            .inventory
            .player_currencies_like_cpp(crate::session::hub_ref(session))
            .expect("unowned currency fixture state");
        let fixture_money = session.player_gold_like_cpp();
        let fixture_inventory_slot_count = session
            .inventory
            .player_inventory_slot_count_for_test_like_cpp();
        let fixture_bank_bag_slot_count = session
            .inventory
            .player_bank_bag_slot_count_for_test_like_cpp();
        let fixture_reputation = session.reputation_mgr_like_cpp().cloned_state_like_cpp();
        let fixture_map_id = session.core.player_map_id_like_cpp();
        ensure_native_quest_log_player_owner_like_cpp(session, fixture_map_id);
        assert!(
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    *player.inventory_runtime_mut_like_cpp() = fixture_inventory;
                    player.install_currencies_like_cpp(fixture_currencies);
                    player.set_money(fixture_money);
                    player.set_inventory_slot_count(fixture_inventory_slot_count);
                    player.set_bank_bag_slot_count(fixture_bank_bag_slot_count);
                })
                .is_some(),
            "native quest fixture Player must retain preconfigured input state"
        );
        assert!(
            session
                .mutate_reputation_mgr_like_cpp(|manager| {
                    manager.replace_state_like_cpp(fixture_reputation)
                })
                .is_some(),
            "native quest fixture Player must retain preconfigured reputation"
        );
        session
            .mutate_player_quest_gameplay_like_cpp(|quests| {
                quests.replace_statuses_like_cpp(
                    existing.statuses_snapshot_like_cpp(),
                    existing.status_authority_complete_like_cpp(),
                );
                quests.replace_rewarded_quest_ids_like_cpp(
                    existing.rewarded_quest_ids_like_cpp().clone(),
                );
                quests.replace_rewarded_quest_rows_like_cpp(
                    existing.rewarded_quest_rows_like_cpp().clone(),
                );
                quests.replace_daily_quest_ids_like_cpp(
                    existing.daily_quest_ids_like_cpp().clone(),
                );
                quests.replace_weekly_quest_ids_like_cpp(
                    existing.weekly_quest_ids_like_cpp().clone(),
                );
                quests.replace_monthly_quest_ids_like_cpp(
                    existing.monthly_quest_ids_like_cpp().clone(),
                );
                quests.replace_seasonal_quests_like_cpp(
                    existing.seasonal_quests_like_cpp().clone(),
                    existing.seasonal_quest_changed_like_cpp(),
                );
                quests.replace_df_quest_ids_like_cpp(existing.df_quest_ids_like_cpp().clone());
                quests.set_last_daily_quest_time_secs_like_cpp(
                    existing.last_daily_quest_time_secs_like_cpp(),
                );
                quests.set_pending_share_like_cpp(fixture_pending_share);
                quests.replace_objective_counts_by_quest_like_cpp(
                    existing.objective_counts_by_quest_like_cpp().to_vec(),
                );
            })
            .expect("installed test Player quest owner");

        for status in existing.statuses_like_cpp().values() {
            if status.status != wow_conditions::QUEST_STATUS_NONE_LIKE_CPP
                && usize::from(status.slot) < 25
            {
                insert_player_quest_status_fixture_like_cpp(session, status.quest_id, status.clone());
            }
        }
    }
}

pub(super) fn install_native_quest_log_bag_fixture_like_cpp(
    session: &mut WorldSession,
    bag_slot: u8,
    bag_guid: wow_core::ObjectGuid,
    bag_size: u8,
    child_item: Option<(u8, wow_core::ObjectGuid)>,
) {
    assert!(
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.store_top_level_item(bag_slot, bag_guid).unwrap();
                player
                    .register_bag_storage(bag_slot, bag_guid, bag_size)
                    .unwrap();
                if let Some((child_slot, child_guid)) = child_item {
                    player.store_bag_item(bag_slot, child_slot, child_guid).unwrap();
                }
            })
            .is_some(),
        "bag placement fixture requires the canonical Player"
    );
}

fn attach_map_manager_like_cpp(
    session: &mut WorldSession,
    mut manager: wow_map::MapManager,
) {
    let player_guid = session.player_guid().expect("quest test Player guid");
    if !manager
        .find_map(571, 0)
        .is_some_and(|map| map.map().get_typed_player(player_guid).is_some())
    {
        let player = session
            .core
            .with_owned_player_like_cpp(|player| player.clone())
            .expect("quest fixture has an owned Player");
        manager
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    session.set_canonical_map_manager(std::sync::Arc::new(std::sync::Mutex::new(manager)));
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
}

fn ensure_quest_log_map_player_like_cpp(session: &mut WorldSession) {
    let Some(manager) = session.core.canonical_map_manager.as_ref().cloned() else {
        attach_map_manager_like_cpp(session, wow_map::MapManager::default());
        return;
    };
    let has_player = manager.lock().ok().is_some_and(|manager| {
        manager.find_map(571, 0).is_some_and(|map| {
            map.map()
                .get_typed_player(session.player_guid().unwrap())
                .is_some()
        })
    });
    if has_player {
        return;
    }
    let player = session
        .core
        .with_owned_player_like_cpp(|player| player.clone())
        .expect("quest fixture has an owned Player");
    manager
        .lock()
        .expect("quest fixture Map manager lock")
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
}

pub(super) fn insert_player_quest_status_fixture_like_cpp(
    session: &mut WorldSession,
    quest_id: u32,
    status: PlayerQuestStatus,
) {
    session
        .quest_state
        .fixture_insert_player_quest_status_like_cpp(quest_id, status.clone());
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.insert_status_like_cpp(quest_id, status.clone());
        })
        .expect("test Player quest owner");

    let owner = session.core.quest_objective_access_like_cpp();
    if owner.owner_handle_absent_like_cpp() {
        return;
    }
    let slot = status.slot;
    assert_eq!(owner.set_quest_slot_like_cpp(slot, quest_id), Some(true));
    let end_time = session
        .catalogs
        .quests
        .store
        .as_ref()
        .and_then(|store| store.get(quest_id))
        .filter(|quest| quest.limit_time_secs > 0)
        .map_or(0, |_| status.end_time_secs);
    assert_eq!(
        owner.set_quest_slot_end_time_like_cpp(slot, end_time),
        Some(true)
    );
    match status.status {
        wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP => {
            assert_eq!(owner.set_quest_slot_state_like_cpp(slot, 1), Some(true));
        }
        wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP => {
            assert_eq!(owner.set_quest_slot_state_like_cpp(slot, 2), Some(true));
        }
        _ => {}
    }
    for (index, count) in status.objective_counts.iter().copied().enumerate().take(24) {
        let stores_flag = session
            .catalogs
            .quests
            .store
            .as_ref()
            .and_then(|store| store.get(quest_id))
            .is_some_and(|quest| {
                quest.objectives.iter().any(|objective| {
                    objective.storage_index == index as i8 && objective.is_storing_flag_like_cpp()
                })
            });
        if stores_flag {
            if count != 0 {
                assert_eq!(
                    owner.set_quest_slot_state_like_cpp(slot, 256u32 << index),
                    Some(true)
                );
            }
        } else {
            assert_eq!(
                owner.set_quest_slot_counter_like_cpp(slot, index as u8, count as u16),
                Some(true)
            );
        }
    }
}

pub(super) fn publish_quest_log_map_values_like_cpp(session: &mut WorldSession) -> usize {
    ensure_quest_log_map_player_like_cpp(session);
    let Some(manager) = session.core.canonical_map_manager.as_ref().cloned() else {
        return 0;
    };
    let Ok(mut manager) = manager.lock() else {
        return 0;
    };
    let _ = manager.update(1);
    drop(manager);
    session.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp()
}
