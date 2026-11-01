// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use crate::binding::player_instance_reset_forgets_recent_like_cpp;
use crate::{InstanceState, RepresentedPendingBind, create_map_instance_lock_token_like_cpp};
use tracing::{info, warn};
use wow_core::ObjectGuid;
use wow_data::{MapDifficultyStore, MapStore};
use wow_map::InstanceResetResultLikeCpp;
use wow_packet::packets::instance::{
    InstanceLockResponse, InstanceReset, InstanceResetFailed, InstanceSaveCreated,
    ResetFailedNotify,
};
use wow_packet::packets::misc::{
    CalendarRaidLockoutAdded, CalendarRaidLockoutUpdated, SetSavedInstanceExtend,
};
use wow_packet::{ClientPacket, ServerPacket};
use wow_persistence::InstanceLockPersistenceOutcomeLikeCpp;
use wow_social::group::{GroupInstanceResetMethodLikeCpp, GroupInstanceResetResultLikeCpp};
use wow_world_core::session::directory::PlayerRegistry;
use wow_world_core::session::{
    GroupDifficultyAccessLikeCpp, InstanceLockManagerAccessLikeCpp, InstancePlayerAccessLikeCpp,
    PacketPublicationAccessLikeCpp, SharedCanonicalMapManager,
};

use crate::handlers::InstanceLockOperationsHandlerCxLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceResetMethodLikeCpp {
    Manual,
    OnChangeDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceLockResponseOutcomeLikeCpp {
    Malformed,
    NoPendingBind,
    Rejected,
    Confirmed(u32),
}

pub async fn handle_reset_instances_like_cpp(
    cx: &mut InstanceLockOperationsHandlerCxLikeCpp<'_>,
    _pkt: wow_packet::WorldPacket,
) {
    let Some(player_guid) = cx.player.player_guid_like_cpp() else {
        return;
    };

    if cx
        .map_store
        .and_then(|store| store.get(u32::from(cx.player.player_map_id_like_cpp())))
        .is_some_and(|map| map.instance_type != 0)
    {
        return;
    }

    let reset_owner_guid = if let Some(group_guid) = cx
        .social
        .resolved_group_guid_with_access_like_cpp(&cx.group_owner, cx.consumer_test)
    {
        let Some(group) = cx.groups.group_snapshot_like_cpp(group_guid) else {
            return;
        };
        if group.leader_guid != player_guid {
            return;
        }
        if group.is_lfg_group_like_cpp() {
            return;
        }
        group.leader_guid
    } else {
        player_guid
    };

    let _ = reset_represented_instances_like_cpp(
        cx,
        reset_owner_guid,
        InstanceResetMethodLikeCpp::Manual,
    );
}

pub fn reset_represented_instances_like_cpp(
    cx: &mut InstanceLockOperationsHandlerCxLikeCpp<'_>,
    reset_owner_guid: ObjectGuid,
    method: InstanceResetMethodLikeCpp,
) -> bool {
    let group_guid = cx
        .social
        .resolved_group_guid_with_access_like_cpp(&cx.group_owner, cx.consumer_test);
    reset_maps_with_access_like_cpp(
        cx.instances,
        &cx.locks,
        &cx.player,
        &cx.groups,
        &cx.packets,
        cx.map_store,
        cx.map_difficulty_store,
        cx.canonical_map_manager,
        cx.player_registry,
        group_guid,
        reset_owner_guid,
        method,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn reset_maps_with_access_like_cpp(
    instances: &mut InstanceState,
    locks: &InstanceLockManagerAccessLikeCpp<'_>,
    player: &InstancePlayerAccessLikeCpp<'_>,
    groups: &GroupDifficultyAccessLikeCpp<'_>,
    packets: &PacketPublicationAccessLikeCpp<'_>,
    map_store: Option<&MapStore>,
    map_difficulty_store: Option<&MapDifficultyStore>,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    player_registry: Option<&Arc<PlayerRegistry>>,
    group_guid: Option<u64>,
    reset_owner_guid: ObjectGuid,
    method: InstanceResetMethodLikeCpp,
) -> bool {
    let keys = if let Some(group_guid) = group_guid {
        let Some(group) = groups.group_snapshot_like_cpp(group_guid) else {
            return false;
        };
        if group.leader_guid != reset_owner_guid {
            return false;
        }
        group
            .owned_instances_like_cpp()
            .map(|owned| (owned.map_id, owned.instance_id))
            .collect::<Vec<_>>()
    } else {
        if player.player_guid_like_cpp() != Some(reset_owner_guid) {
            return false;
        }
        let Some(keys) = recent_instances_snapshot_like_cpp(instances, player) else {
            return false;
        };
        keys
    };
    let group_method = match method {
        InstanceResetMethodLikeCpp::Manual => GroupInstanceResetMethodLikeCpp::Manual,
        InstanceResetMethodLikeCpp::OnChangeDifficulty => {
            GroupInstanceResetMethodLikeCpp::OnChangeDifficulty
        }
    };

    for (map_id, instance_id) in keys {
        let result = reset_live_map_instance_like_cpp(
            locks,
            canonical_map_manager,
            map_store,
            map_difficulty_store,
            map_id,
            instance_id,
        );
        let group_result = match &result {
            InstanceResetResultLikeCpp::Success => GroupInstanceResetResultLikeCpp::Success,
            InstanceResetResultLikeCpp::NotEmpty { .. } => {
                GroupInstanceResetResultLikeCpp::NotEmpty
            }
            InstanceResetResultLikeCpp::CannotReset => GroupInstanceResetResultLikeCpp::CannotReset,
            InstanceResetResultLikeCpp::Other => GroupInstanceResetResultLikeCpp::Other,
        };

        match result {
            InstanceResetResultLikeCpp::Success => {
                packets.send_packet(&InstanceReset { map_id });
            }
            InstanceResetResultLikeCpp::NotEmpty { occupant_guids }
                if method == InstanceResetMethodLikeCpp::Manual =>
            {
                notify_reset_failed_occupants_like_cpp(player_registry, &occupant_guids);
                packets.send_packet(&InstanceResetFailed {
                    map_id,
                    reset_failed_reason: 0,
                });
            }
            InstanceResetResultLikeCpp::NotEmpty { .. }
            | InstanceResetResultLikeCpp::CannotReset
            | InstanceResetResultLikeCpp::Other => {}
        }

        if let Some(group_guid) = group_guid {
            let _ = groups.apply_instance_reset_transition_like_cpp(
                group_guid,
                map_id,
                group_result,
                group_method,
            );
        } else if player_instance_reset_forgets_recent_like_cpp(group_result, group_method) {
            let _ = forget_recent_instance_like_cpp(instances, player, map_id);
        }
    }

    true
}

fn reset_live_map_instance_like_cpp(
    locks: &InstanceLockManagerAccessLikeCpp<'_>,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    map_store: Option<&MapStore>,
    map_difficulty_store: Option<&MapDifficultyStore>,
    map_id: u32,
    instance_id: u32,
) -> InstanceResetResultLikeCpp {
    let Some(canonical_map_manager) = canonical_map_manager else {
        return InstanceResetResultLikeCpp::Other;
    };
    let snapshot = {
        let Ok(manager) = canonical_map_manager.lock() else {
            return InstanceResetResultLikeCpp::Other;
        };
        manager.instance_reset_snapshot_like_cpp(map_id, instance_id)
    };
    let Some(snapshot) = snapshot else {
        return InstanceResetResultLikeCpp::Other;
    };

    let lock_is_new = match snapshot.instance_lock_context {
        None => None,
        Some(context) => {
            let Some((lock, entries)) = locks.active_lock_for_map_reset_like_cpp(
                context,
                map_id,
                instance_id,
                snapshot.difficulty_id,
                map_store,
                map_difficulty_store,
            ) else {
                return InstanceResetResultLikeCpp::Other;
            };
            let Ok(owner_guid_counter) = i64::try_from(context.owner_guid_counter) else {
                return InstanceResetResultLikeCpp::Other;
            };
            let owner_guid = ObjectGuid::create_player(1, owner_guid_counter);
            if create_map_instance_lock_token_like_cpp(owner_guid, &entries, &lock) != context.token
            {
                return InstanceResetResultLikeCpp::Other;
            }
            Some(lock.is_new)
        }
    };

    let Ok(mut manager) = canonical_map_manager.lock() else {
        return InstanceResetResultLikeCpp::Other;
    };
    manager.reset_instance_at_incarnation_like_cpp(
        map_id,
        instance_id,
        snapshot.incarnation,
        lock_is_new,
    )
}

fn notify_reset_failed_occupants_like_cpp(
    player_registry: Option<&Arc<PlayerRegistry>>,
    occupant_guids: &[ObjectGuid],
) {
    let Some(player_registry) = player_registry else {
        return;
    };
    let packet = ResetFailedNotify.to_bytes();
    for occupant_guid in occupant_guids {
        let Some(registration) = player_registry.current_registration_like_cpp(*occupant_guid)
        else {
            continue;
        };
        let _ = player_registry.send_current_packet(registration, packet.clone());
    }
}

fn recent_instances_snapshot_like_cpp(
    instances: &InstanceState,
    player: &InstancePlayerAccessLikeCpp<'_>,
) -> Option<Vec<(u32, u32)>> {
    if let Some(snapshot) = player.recent_instances_snapshot_like_cpp() {
        return Some(snapshot);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if player.owner_handle_absent_like_cpp() {
        return Some(instances.represented_recent_instances_snapshot_for_handler_like_cpp());
    }
    None
}

fn forget_recent_instance_like_cpp(
    instances: &mut InstanceState,
    player: &InstancePlayerAccessLikeCpp<'_>,
    map_id: u32,
) -> bool {
    if let Some(forgot) = player.forget_recent_instance_like_cpp(map_id) {
        return forgot;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if player.owner_handle_absent_like_cpp() {
        return instances.forget_represented_recent_instance_for_handler_like_cpp(map_id);
    }
    false
}

pub async fn handle_instance_lock_response_like_cpp(
    cx: &mut InstanceLockOperationsHandlerCxLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) -> InstanceLockResponseOutcomeLikeCpp {
    let Ok(response) = InstanceLockResponse::read(&mut pkt) else {
        return InstanceLockResponseOutcomeLikeCpp::Malformed;
    };

    let Some(pending_bind) = cx.instances.take_pending_bind_like_cpp() else {
        info!(
            account = cx.player.account_id_like_cpp(),
            player_guid = ?cx.player.player_guid_like_cpp(),
            "InstanceLockResponse without pending bind"
        );
        return InstanceLockResponseOutcomeLikeCpp::NoPendingBind;
    };

    if !response.accept_lock {
        if cx.consumer_test {
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                *cx.rejected_response_counter = (*cx.rejected_response_counter).saturating_add(1);
            }
        }
        return InstanceLockResponseOutcomeLikeCpp::Rejected;
    }

    if confirm_pending_bind_like_cpp(cx, pending_bind).await {
        if cx.consumer_test {
            #[cfg(any(test, feature = "test-fixtures"))]
            cx.instances
                .record_represented_confirmed_pending_bind_for_test_like_cpp(
                    pending_bind.instance_id,
                );
        }
        InstanceLockResponseOutcomeLikeCpp::Confirmed(pending_bind.instance_id)
    } else {
        InstanceLockResponseOutcomeLikeCpp::Rejected
    }
}

async fn confirm_pending_bind_like_cpp(
    cx: &mut InstanceLockOperationsHandlerCxLikeCpp<'_>,
    pending_bind: RepresentedPendingBind,
) -> bool {
    if u32::from(cx.player.player_map_id_like_cpp()) != pending_bind.map_id {
        return false;
    }

    let Some(difficulty_id) = cx
        .player
        .canonical_map_difficulty_like_cpp(pending_bind.map_id, pending_bind.instance_id)
    else {
        return false;
    };

    let Some(is_game_master) = cx.game_master_like_cpp() else {
        return false;
    };
    if is_game_master {
        return true;
    }

    let Some(player_guid) = cx.player.player_guid_like_cpp() else {
        return false;
    };
    let Some(entries) = map_db2_entries_like_cpp(
        cx.instances,
        cx.map_store,
        cx.map_difficulty_store,
        cx.difficulty_store,
        pending_bind.map_id,
        difficulty_id.into(),
    ) else {
        return false;
    };
    let Some((is_new_lock, new_lock, persistence_plan, now)) =
        cx.locks.update_lock_for_player_with_persistence_like_cpp(
            player_guid,
            &entries,
            pending_bind.instance_id,
            pending_bind.completed_mask,
        )
    else {
        return false;
    };

    if !persistence_plan.is_empty()
        && let Some(port) = cx.lifecycle.instance_lock_persistence_port_like_cpp()
        && let InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
            port.commit_plan_like_cpp(persistence_plan).await
    {
        warn!(
            account = cx.player.account_id_like_cpp(),
            player_guid = ?player_guid,
            instance_id = pending_bind.instance_id,
            error = %reason,
            "failed to commit represented pending instance bind transaction"
        );
        return false;
    }

    if is_new_lock {
        cx.packets
            .send_packet(&InstanceSaveCreated { gm: is_game_master });
        let reset_schedule = cx.locks.reset_schedule_like_cpp();
        send_calendar_raid_lockout_added_like_cpp(
            &cx.packets,
            &new_lock,
            &entries,
            reset_schedule,
            now,
        );
    }

    true
}

fn send_calendar_raid_lockout_added_like_cpp(
    packets: &PacketPublicationAccessLikeCpp<'_>,
    lock: &wow_instances::InstanceLock,
    entries: &wow_instances::MapDb2Entries,
    reset_schedule: wow_instances::ResetSchedule,
    now: u64,
) {
    let effective_expiry = lock.effective_expiry_time_at(entries, reset_schedule, now);
    let remaining = (effective_expiry as i128 - now as i128)
        .clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
    packets.send_packet(&CalendarRaidLockoutAdded::new_at_unix(
        u64::from(lock.instance_id),
        now.min(i64::MAX as u64) as i64,
        i32::try_from(lock.map_id).unwrap_or(i32::MAX),
        u32::from(lock.difficulty_id),
        remaining,
    ));
}

pub async fn handle_set_saved_instance_extend_like_cpp(
    cx: &mut InstanceLockOperationsHandlerCxLikeCpp<'_>,
    query: SetSavedInstanceExtend,
) {
    let Some(player_guid) = cx.player.player_guid_like_cpp() else {
        return;
    };

    let Ok(map_id) = u32::try_from(query.map_id) else {
        return;
    };
    if u32::from(cx.player.player_map_id_like_cpp()) == map_id {
        return;
    }

    let Ok(difficulty_id) = wow_map::Difficulty::try_from(query.difficulty_id) else {
        return;
    };
    let Some(entries) = map_db2_entries_like_cpp(
        cx.instances,
        cx.map_store,
        cx.map_difficulty_store,
        cx.difficulty_store,
        map_id,
        difficulty_id,
    ) else {
        return;
    };
    let Some((expiry_times, persistence_plan, now)) = cx
        .locks
        .update_lock_extension_with_persistence_like_cpp(player_guid, &entries, query.extend)
    else {
        return;
    };

    if !persistence_plan.is_empty()
        && let Some(port) = cx.lifecycle.instance_lock_persistence_port_like_cpp()
        && let InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
            port.commit_plan_like_cpp(persistence_plan).await
    {
        warn!(
            account = cx.player.account_id_like_cpp(),
            player_guid = ?player_guid,
            map_id,
            difficulty_id,
            error = %reason,
            "failed to commit represented instance lock extension transaction"
        );
        return;
    }

    let remaining = |expiry: u64| -> i32 {
        (expiry.saturating_sub(now) as i128)
            .min(i128::from(i32::MAX))
            .max(0) as i32
    };
    cx.packets
        .send_packet(&CalendarRaidLockoutUpdated::new_at_unix(
            now.min(i64::MAX as u64) as i64,
            query.map_id,
            query.difficulty_id,
            remaining(expiry_times.0),
            remaining(expiry_times.1),
        ));
}

fn map_db2_entries_like_cpp(
    instances: &InstanceState,
    map_store: Option<&MapStore>,
    map_difficulty_store: Option<&MapDifficultyStore>,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    map_id: u32,
    difficulty_id: wow_map::Difficulty,
) -> Option<wow_instances::MapDb2Entries> {
    instances.create_map_db2_entries_from_stores_like_cpp(
        map_store?,
        map_difficulty_store?,
        difficulty_store?,
        map_id,
        difficulty_id,
    )
}
