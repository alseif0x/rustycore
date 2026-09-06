// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private instance capability handlers extracted from the legacy misc owner.

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketProcessing, SessionStatus};
use wow_persistence::{InstanceLockPersistenceOutcomeLikeCpp, InstanceLockPersistencePlanLikeCpp};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::instance::{
    InstanceInfo, InstanceLockInfo, InstanceLockResponse, InstanceReset, InstanceResetFailed,
    InstanceSaveCreated, PendingRaidLock,
};
use wow_packet::packets::misc::{
    CalendarRaidLockoutAdded, CalendarRaidLockoutUpdated, SetSavedInstanceExtend,
};

use super::RepresentedInstanceResetMethodLikeCpp;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRaidInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_raid_info",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_raid_info(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ResetInstances,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_reset_instances",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_reset_instances(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::InstanceLockResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_instance_lock_response",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_instance_lock_response(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_request_raid_info(&mut self, _pkt: wow_packet::WorldPacket) {
        let locks = match (self.player_guid(), self.instance_lock_mgr.as_ref()) {
            (Some(player_guid), Some(instance_lock_mgr)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                instance_lock_mgr
                    .read()
                    .map(|mgr| {
                        let map_store = self.map_store().map(|store| store.as_ref());
                        let map_difficulty_store =
                            self.map_difficulty_store().map(|store| store.as_ref());
                        mgr.get_raid_info_locks_for_player_at(
                            player_guid,
                            now,
                            wow_instances::ResetSchedule::default(),
                            |map_id, difficulty_id| {
                                let map = map_store?.get(map_id)?;
                                let map_difficulty =
                                    map_difficulty_store?.get(map_id, difficulty_id)?;
                                Some(wow_instances::MapDb2Entries {
                                    map_id,
                                    difficulty_id,
                                    lock_id: u32::from(map_difficulty.lock_id),
                                    reset_interval: match map_difficulty.reset_interval {
                                        1 => wow_instances::MapDifficultyResetInterval::Daily,
                                        2 => wow_instances::MapDifficultyResetInterval::Weekly,
                                        _ => wow_instances::MapDifficultyResetInterval::Anytime,
                                    },
                                    max_players: map_difficulty.max_players,
                                    is_flex_locking: map.is_flex_locking(),
                                    is_using_encounter_locks: map_difficulty
                                        .is_using_encounter_locks(),
                                })
                            },
                        )
                    })
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };

        self.send_packet_realm(&InstanceInfo {
            locks: locks
                .into_iter()
                .map(|lock| InstanceLockInfo {
                    instance_id: lock.instance_id,
                    map_id: lock.map_id,
                    difficulty_id: lock.difficulty_id,
                    time_remaining: lock.time_remaining,
                    completed_mask: lock.completed_mask,
                    locked: lock.locked,
                    extended: lock.extended,
                })
                .collect(),
        });
    }

    /// C++ `WorldSession::HandleResetInstancesOpcode`.

    pub async fn handle_reset_instances(&mut self, _pkt: wow_packet::WorldPacket) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if self
            .map_store()
            .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
            .is_some_and(|map| map.instance_type != 0)
        {
            return;
        }

        let reset_owner_guid = if let Some(group_guid) = self.resolved_group_guid_like_cpp() {
            let Some(group_registry) = self.group_registry() else {
                return;
            };
            let Some(group) = group_registry.get(&group_guid) else {
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

        let _ = self
            .reset_represented_instances_like_cpp(
                reset_owner_guid,
                RepresentedInstanceResetMethodLikeCpp::Manual,
            )
            .await;
    }

    pub(super) async fn reset_represented_instances_like_cpp(
        &mut self,
        reset_owner_guid: ObjectGuid,
        method: RepresentedInstanceResetMethodLikeCpp,
    ) -> bool {
        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return false;
        };

        let mut persistence_plan = InstanceLockPersistencePlanLikeCpp::default();
        let reset_result = {
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return false,
            };
            let entries_by_key = mgr
                .player_lock_map_difficulties(reset_owner_guid)
                .into_iter()
                .filter_map(|(map_id, difficulty_id)| {
                    let map = self.map_store()?.get(map_id)?;
                    let map_difficulty = self.map_difficulty_store()?.get(map_id, difficulty_id)?;
                    let entries = wow_instances::MapDb2Entries {
                        map_id,
                        difficulty_id,
                        lock_id: u32::from(map_difficulty.lock_id),
                        reset_interval: match map_difficulty.reset_interval {
                            1 => wow_instances::MapDifficultyResetInterval::Daily,
                            2 => wow_instances::MapDifficultyResetInterval::Weekly,
                            _ => wow_instances::MapDifficultyResetInterval::Anytime,
                        },
                        max_players: map_difficulty.max_players,
                        is_flex_locking: map.is_flex_locking(),
                        is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
                    };
                    Some((entries.key(), entries))
                })
                .collect::<std::collections::HashMap<_, _>>();

            mgr.reset_instance_locks_for_player_with_persistence_at(
                &mut persistence_plan,
                reset_owner_guid,
                None,
                None,
                &entries_by_key,
                wow_instances::ResetSchedule::default(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0),
            )
        };

        if !persistence_plan.is_empty() {
            if let Some(port) = self.instance_lock_persistence_port_like_cpp()
                && let InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
                    port.commit_plan_like_cpp(persistence_plan).await
            {
                warn!(
                    account = self.account_id,
                    player_guid = ?reset_owner_guid,
                    error = %reason,
                    "failed to commit represented instance lock reset transaction"
                );
                return false;
            }
        }

        for lock in reset_result.reset {
            self.send_packet(&InstanceReset {
                map_id: lock.map_id,
            });
        }

        if method == RepresentedInstanceResetMethodLikeCpp::Manual {
            for lock in reset_result.failed_to_reset {
                self.send_packet(&InstanceResetFailed {
                    map_id: lock.map_id,
                    reset_failed_reason: 0,
                });
            }
        }

        true
    }

    /// C++ `WorldSession::HandleInstanceLockResponse`.

    pub async fn handle_instance_lock_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let Ok(response) = InstanceLockResponse::read(&mut pkt) else {
            return;
        };

        let Some(pending_bind) = self.pending_bind.take() else {
            info!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "InstanceLockResponse without pending bind"
            );
            return;
        };

        if response.accept_lock {
            if self.confirm_pending_bind_like_cpp(pending_bind).await {
                self.represented_confirmed_pending_binds
                    .push(pending_bind.instance_id);
            }
        } else {
            #[cfg(test)]
            {
                self.represented_repop_at_graveyard_count =
                    self.represented_repop_at_graveyard_count.saturating_add(1);
            }
        }
    }

    /// Represented C++ `Player::ConfirmPendingBind`.
    ///
    /// The real C++ path asks the current `InstanceMap` to create a player lock
    /// only when the player's current map instance matches `_pendingBindId`.
    /// Rust does not own full `InstanceMap::i_data` yet, so this bridge uses the
    /// pending-lock completed mask that produced `SMSG_PENDING_RAID_LOCK` as the
    /// available represented `i_instanceLock->GetData()` state.
    async fn confirm_pending_bind_like_cpp(
        &mut self,
        pending_bind: crate::session::RepresentedPendingBind,
    ) -> bool {
        if u32::from(self.player_map_id_like_cpp()) != pending_bind.map_id {
            return false;
        }

        let difficulty_id = {
            let Some(manager) = self.canonical_map_manager.as_ref() else {
                return false;
            };
            let Ok(manager) = manager.lock() else {
                return false;
            };
            let Some(map) = manager.find_map(pending_bind.map_id, pending_bind.instance_id) else {
                return false;
            };
            map.difficulty()
        };

        let Some(is_game_master) = self.player_is_game_master_like_cpp() else {
            return false;
        };
        if is_game_master {
            return true;
        }

        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(entries) =
            self.create_map_db2_entries_like_cpp(pending_bind.map_id, difficulty_id)
        else {
            return false;
        };
        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return false;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut persistence_plan = InstanceLockPersistencePlanLikeCpp::default();
        let (is_new_lock, new_lock) = {
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return false,
            };
            let is_new_lock = mgr
                .find_active_instance_lock_at(player_guid, &entries, now)
                .is_none_or(|lock| lock.is_new || lock.is_expired_at(now));
            let update_event = wow_instances::InstanceLockUpdateEvent {
                instance_id: pending_bind.instance_id,
                new_data: String::new(),
                instance_completed_encounters_mask: pending_bind.completed_mask,
                completed_encounter_bit: None,
                entrance_world_safe_loc_id: None,
            };
            let Some(new_lock) = mgr.update_instance_lock_for_player_with_persistence_at(
                &mut persistence_plan,
                player_guid,
                &entries,
                update_event,
                self.reset_schedule_like_cpp(),
                now,
            ) else {
                return false;
            };
            (is_new_lock, new_lock)
        };

        if !persistence_plan.is_empty() {
            if let Some(port) = self.instance_lock_persistence_port_like_cpp()
                && let InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
                    port.commit_plan_like_cpp(persistence_plan).await
            {
                warn!(
                    account = self.account_id,
                    player_guid = ?player_guid,
                    instance_id = pending_bind.instance_id,
                    error = %reason,
                    "failed to commit represented pending instance bind transaction"
                );
                return false;
            }
        }

        if is_new_lock {
            self.send_packet(&InstanceSaveCreated { gm: is_game_master });
            self.send_calendar_raid_lockout_added_like_cpp(&new_lock, &entries, now);
        }

        true
    }

    /// C++ `WorldSession::SendCalendarRaidLockoutAdded`.
    fn send_calendar_raid_lockout_added_like_cpp(
        &self,
        lock: &wow_instances::InstanceLock,
        entries: &wow_instances::MapDb2Entries,
        now: u64,
    ) {
        let effective_expiry =
            lock.effective_expiry_time_at(entries, self.reset_schedule_like_cpp(), now);
        let remaining = (effective_expiry as i128 - now as i128)
            .clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
        self.send_packet(&CalendarRaidLockoutAdded::new_at_unix(
            u64::from(lock.instance_id),
            now.min(i64::MAX as u64) as i64,
            i32::try_from(lock.map_id).unwrap_or(i32::MAX),
            u32::from(lock.difficulty_id),
            remaining,
        ));
    }

    #[allow(dead_code)]
    pub(crate) fn send_pending_raid_lock_like_cpp(
        &mut self,
        instance_id: u32,
        completed_mask: u32,
        extending: bool,
        warning_only: bool,
    ) {
        self.send_packet(&PendingRaidLock {
            time_until_lock: 60_000,
            completed_mask,
            extending,
            warning_only,
        });

        if !warning_only {
            self.pending_bind = Some(crate::session::RepresentedPendingBind {
                map_id: u32::from(self.player_map_id_like_cpp()),
                instance_id,
                completed_mask,
                time_until_lock_ms: 60_000,
            });
        }
    }

    /// C++ `WorldSession::HandleSetSavedInstanceExtend`.
    pub async fn handle_set_saved_instance_extend(&mut self, query: SetSavedInstanceExtend) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let Ok(map_id) = u32::try_from(query.map_id) else {
            return;
        };
        if u32::from(self.player_map_id_like_cpp()) == map_id {
            return;
        }

        let Ok(difficulty_id) = wow_map::Difficulty::try_from(query.difficulty_id) else {
            return;
        };
        let Some(entries) = self.create_map_db2_entries_like_cpp(map_id, difficulty_id) else {
            return;
        };
        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut persistence_plan = InstanceLockPersistencePlanLikeCpp::default();
        let Some((old_expiry, new_expiry)) = ({
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return,
            };
            mgr.update_instance_lock_extension_for_player_with_persistence_at(
                &mut persistence_plan,
                player_guid,
                &entries,
                query.extend,
                self.reset_schedule_like_cpp(),
                now,
            )
        }) else {
            return;
        };

        if !persistence_plan.is_empty()
            && let Some(port) = self.instance_lock_persistence_port_like_cpp()
            && let InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
                port.commit_plan_like_cpp(persistence_plan).await
        {
            warn!(
                account = self.account_id,
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
        self.send_packet(&CalendarRaidLockoutUpdated::new_at_unix(
            now.min(i64::MAX as u64) as i64,
            query.map_id,
            query.difficulty_id,
            remaining(old_expiry),
            remaining(new_expiry),
        ));
    }
}
