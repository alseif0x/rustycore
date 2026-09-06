//! Map selection/admission and final Player attachment are separate synchronous phases.
//! C++ MapManager::CreateMap (MapManager.cpp:139-231) returns a map;
//! Player/WorldSession owns the subsequent binding and AddPlayerToMap.
//! The compatibility facade preserves the existing Rust order and failure results.
use crate::session::{
    TRANSFER_ABORT_DIFFICULTY_LIKE_CPP, TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP,
    TRANSFER_ABORT_NEED_GROUP_LIKE_CPP, TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP,
    TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP, WorldSession,
    create_map_decision_difficulty_id_like_cpp, unix_now,
};
use std::sync::Arc;
use tracing::warn;

impl WorldSession {
    /// Synchronous entry attempt. Failure retains the source coordinates and
    /// transfer state; preparation can still create maps/admission side effects.
    /// Full C++ before-add publication ordering and homebind recovery remain
    /// separate obligations; this is not a reservation across asynchronous work.
    pub(crate) fn try_attach_worldport_destination_like_cpp(
        &mut self,
        map_id: u32,
        position: wow_core::Position,
    ) -> bool {
        let Ok(map_id_u16) = u16::try_from(map_id) else {
            return false;
        };
        if !position.is_valid_map_coord_like_cpp() {
            return false;
        }
        let Some(
            wow_map::CreateMapDecision::Existing { key, .. }
            | wow_map::CreateMapDecision::Create { key, .. },
        ) = self.prepare_canonical_map_entry_like_cpp(map_id)
        else {
            return false;
        };
        if !self.remove_current_player_from_canonical_current_map_like_cpp()
            || !self.ensure_canonical_player_owner_for_map_like_cpp(key, position)
        {
            return false;
        }
        self.set_player_map_position_like_cpp(map_id_u16, position);
        true
    }

    pub(crate) fn ensure_canonical_world_map_for_current_player_like_cpp(
        &mut self,
    ) -> Option<wow_map::CreateMapDecision> {
        let map_id = u32::from(self.player_map_id_like_cpp());
        let position = self.player_position_like_cpp()?;
        let decision = self.prepare_canonical_map_entry_like_cpp(map_id)?;
        if let wow_map::CreateMapDecision::Existing { key, .. }
        | wow_map::CreateMapDecision::Create { key, .. } = &decision
            && !self.ensure_canonical_player_owner_for_map_like_cpp(*key, position)
        {
            return None;
        }
        Some(decision)
    }

    /// Prepare/create the destination and its existing admission side effects,
    /// without relocating or attaching Player. This is not a reservation or a
    /// pure query: callers must not carry this decision across asynchronous work.
    pub(in crate::session) fn prepare_canonical_map_entry_like_cpp(
        &mut self,
        map_id: u32,
    ) -> Option<wow_map::CreateMapDecision> {
        let map_entry = self.map_store.as_ref()?.get(map_id).copied()?;
        if map_entry.is_battleground_or_arena() {
            return None;
        }
        // C++ `Player::LoadFromDB` rejects a saved map requiring a newer
        // client expansion before calling `MapManager::CreateMap`
        // (Player.cpp:17577-17587). The login handler observes the missing
        // canonical key and performs the same homebind recovery.
        if self.expansion < map_entry.expansion_like_cpp() {
            warn!(
                account = self.account_id,
                map_id,
                session_expansion = self.expansion,
                required_expansion = map_entry.expansion_like_cpp(),
                "Login map rejected by C++ client expansion gate"
            );
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }

        let player_guid = self.player_guid?;
        let player = self.create_map_player_context_like_cpp(map_id, map_entry, player_guid)?;
        let is_dungeon = map_entry.is_dungeon();
        let requested_difficulty = player
            .group
            .map(|group| group.difficulty_id)
            .unwrap_or(player.player_difficulty_id);
        if is_dungeon
            && self
                .create_map_db2_entries_like_cpp(map_id, requested_difficulty)
                .is_none()
        {
            self.send_transfer_aborted_like_cpp(map_id, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP);
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }
        let bypass_player_cannot_enter_like_cpp =
            is_dungeon && self.player_is_game_master_like_cpp() == Some(true);
        if is_dungeon
            && !bypass_player_cannot_enter_like_cpp
            && let Some((transfer_abort, arg, map_difficulty_x_condition_id)) =
                self.access_requirement_abort_like_cpp(map_id, requested_difficulty as u8)
        {
            self.send_transfer_aborted_with_params_like_cpp(
                map_id,
                transfer_abort,
                arg,
                map_difficulty_x_condition_id,
            );
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }
        if is_dungeon
            && !bypass_player_cannot_enter_like_cpp
            && map_entry.instance_type == wow_data::map::MAP_RAID
            && map_entry.expansion_like_cpp() >= self.server_expansion_like_cpp
            && !self.instance_ignore_raid_like_cpp
            && !self.current_player_is_in_raid_group_like_cpp()
        {
            self.send_transfer_aborted_like_cpp(map_id, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP);
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }
        // Login follows C++ `MapManager::CreateMap` (MapManager.cpp:139-231),
        // whose final world-map branch includes garrisons and selects only
        // instance 0 / team split. Do not use Rust's represented
        // `CreateMapEntryKind::Garrison` here: that mirrors the distinct
        // `FindInstanceIdForPlayer` branch (MapManager.cpp:247-288), which is
        // not the map-creation path called by `Player::LoadFromDB`.
        let entry = wow_map::CreateMapEntryContext {
            map_id,
            kind: if is_dungeon {
                wow_map::CreateMapEntryKind::Dungeon
            } else {
                wow_map::CreateMapEntryKind::World
            },
            split_by_faction: map_entry.is_split_by_faction(),
            flex_locking: map_entry.is_flex_locking(),
        };
        let active_instance_lock = is_dungeon
            .then(|| {
                self.create_map_active_instance_lock_context_like_cpp(map_id, requested_difficulty)
            })
            .flatten();

        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let decision = manager.create_map_decision_like_cpp(
            Some(entry),
            Some(player),
            |map_id, difficulty_id| {
                self.create_map_difficulty_context_like_cpp(map_id, difficulty_id)
            },
            active_instance_lock,
            |_, _| None,
        );

        if let wow_map::CreateMapDecision::Create {
            key,
            difficulty_id,
            kind,
            ..
        } = &decision
        {
            let map = manager.create_map_entry(key.map_id, key.instance_id, *difficulty_id, *kind);
            if is_dungeon {
                map.set_instance_lock_context(active_instance_lock);
            }
        }

        let key = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. }
            | wow_map::CreateMapDecision::Create { key, .. } => Some(*key),
            wow_map::CreateMapDecision::Reject { .. } => None,
        };
        let existing_instance_lock_context = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. } if is_dungeon => manager
                .find_map(key.map_id, key.instance_id)
                .and_then(|map| map.instance_lock_context()),
            _ => None,
        };
        let existing_instance_player_count = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. } if is_dungeon => manager
                .find_map(key.map_id, key.instance_id)
                .map(|map| map.players_count_except_gms_like_cpp()),
            _ => None,
        };
        let existing_instance_encounter_in_progress = match &decision {
            wow_map::CreateMapDecision::Existing { key, .. } if is_dungeon => manager
                .find_map(key.map_id, key.instance_id)
                .map(|map| map.instance_encounter_in_progress_like_cpp()),
            _ => None,
        };
        drop(manager);

        if is_dungeon
            && !bypass_player_cannot_enter_like_cpp
            && let wow_map::CreateMapDecision::Existing {
                key, difficulty_id, ..
            } = &decision
            && let Some(player_count) = existing_instance_player_count
            && let Some(entries) = self.create_map_db2_entries_like_cpp(key.map_id, *difficulty_id)
            && player_count >= entries.max_players
        {
            self.send_transfer_aborted_like_cpp(key.map_id, TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP);
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }

        if is_dungeon
            && !bypass_player_cannot_enter_like_cpp
            && map_entry.instance_type == wow_data::map::MAP_RAID
            && self.player_loading() != Some(player_guid)
            && let wow_map::CreateMapDecision::Existing { key, .. } = &decision
            && existing_instance_encounter_in_progress == Some(true)
        {
            self.send_transfer_aborted_like_cpp(key.map_id, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP);
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }

        if is_dungeon
            && !bypass_player_cannot_enter_like_cpp
            && let wow_map::CreateMapDecision::Existing {
                key, difficulty_id, ..
            } = &decision
            && let Some(lock_context) = existing_instance_lock_context
        {
            let deny_reason = self
                .cannot_enter_existing_instance_lock_like_cpp(
                    key.map_id,
                    *difficulty_id,
                    lock_context,
                )
                .unwrap_or(wow_instances::TransferAbortReason::None);
            if deny_reason != wow_instances::TransferAbortReason::None {
                self.send_transfer_aborted_like_cpp(key.map_id, deny_reason as u32);
                return Some(wow_map::CreateMapDecision::Reject {
                    side_effects: Vec::new(),
                });
            }
        }

        if is_dungeon
            && !bypass_player_cannot_enter_like_cpp
            && !map_entry.ignores_instance_farm_limit_like_cpp()
            && let Some(key) = key
            && !self.check_instance_count_like_cpp(key.instance_id)
            && self.resolved_player_is_alive_like_cpp() == Some(true)
        {
            self.send_transfer_aborted_like_cpp(
                key.map_id,
                TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP,
            );
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }

        if is_dungeon && let Some(key) = key {
            let now_secs = u64::try_from(unix_now()).unwrap_or(0);
            self.add_instance_enter_time_like_cpp(key.instance_id, now_secs);
        }

        // C++ `MapManager::CreateMap` receives the live `Player*` and applies
        // `SetRecentInstance` to that owner before the caller adds it to the
        // selected map (MapManager.cpp:139-231). Materialize the canonical
        // owner before applying those side effects, but keep the actual map
        // transfer until every side effect and lock context is installed.
        if let Some(key) = key
            && !self.ensure_canonical_player_owner_exists_like_cpp(key)
        {
            return None;
        }

        let _ = self.apply_create_map_side_effects_like_cpp(map_id, &decision);

        if is_dungeon
            && let Some(key) = key
            && let Some(difficulty_id) = create_map_decision_difficulty_id_like_cpp(&decision)
            && let Some(lock) =
                self.create_map_active_instance_lock_context_like_cpp(map_id, difficulty_id)
        {
            let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
            if let Ok(mut manager) = manager.lock()
                && let Some(map) = manager.find_map_mut(key.map_id, key.instance_id)
            {
                map.set_instance_lock_context(Some(lock));
            }
        }

        Some(decision)
    }
}
