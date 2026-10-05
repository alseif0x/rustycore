// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use tracing::{info, warn};
use wow_core::ObjectGuid;
use wow_data::{MapDifficultyStore, MapStore};
use wow_packet::ClientPacket;
use wow_packet::packets::instance::{
    InstanceLockResponse, InstanceReset, InstanceResetFailed, InstanceSaveCreated,
};
use wow_packet::packets::misc::{
    CalendarRaidLockoutAdded, CalendarRaidLockoutUpdated, SetSavedInstanceExtend,
};
use wow_persistence::InstanceLockPersistenceOutcomeLikeCpp;
use wow_world_core::session::{
    InstanceLockManagerAccessLikeCpp, InstancePlayerAccessLikeCpp, PacketPublicationAccessLikeCpp,
};
use wow_world_instances::{InstanceState, RepresentedPendingBind};
use wow_world_lifecycle::SessionLifecycleState;

use crate::instances::InstanceLockOperationsHandlerCxLikeCpp;

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

    let _ = reset_locks_with_access_like_cpp(
        &cx.locks,
        cx.lifecycle,
        &cx.packets,
        cx.map_store,
        cx.map_difficulty_store,
        &cx.player,
        reset_owner_guid,
        InstanceResetMethodLikeCpp::Manual,
    )
    .await;
}

pub async fn reset_represented_instances_like_cpp(
    cx: &mut InstanceLockOperationsHandlerCxLikeCpp<'_>,
    reset_owner_guid: ObjectGuid,
    method: InstanceResetMethodLikeCpp,
) -> bool {
    reset_locks_with_access_like_cpp(
        &cx.locks,
        cx.lifecycle,
        &cx.packets,
        cx.map_store,
        cx.map_difficulty_store,
        &cx.player,
        reset_owner_guid,
        method,
    )
    .await
}

pub(super) async fn reset_locks_with_access_like_cpp(
    locks: &InstanceLockManagerAccessLikeCpp<'_>,
    lifecycle: &SessionLifecycleState,
    packets: &PacketPublicationAccessLikeCpp<'_>,
    map_store: Option<&MapStore>,
    map_difficulty_store: Option<&MapDifficultyStore>,
    player: &InstancePlayerAccessLikeCpp<'_>,
    reset_owner_guid: ObjectGuid,
    method: InstanceResetMethodLikeCpp,
) -> bool {
    let Some((reset_result, persistence_plan)) = locks.reset_locks_with_persistence_like_cpp(
        reset_owner_guid,
        map_store,
        map_difficulty_store,
    ) else {
        return false;
    };

    if !persistence_plan.is_empty()
        && let Some(port) = lifecycle.instance_lock_persistence_port_like_cpp()
        && let InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
            port.commit_plan_like_cpp(persistence_plan).await
    {
        warn!(
            account = player.account_id_like_cpp(),
            player_guid = ?reset_owner_guid,
            error = %reason,
            "failed to commit represented instance lock reset transaction"
        );
        return false;
    }

    for lock in reset_result.reset {
        packets.send_packet(&InstanceReset {
            map_id: lock.map_id,
        });
    }

    if method == InstanceResetMethodLikeCpp::Manual {
        for lock in reset_result.failed_to_reset {
            packets.send_packet(&InstanceResetFailed {
                map_id: lock.map_id,
                reset_failed_reason: 0,
            });
        }
    }

    true
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
