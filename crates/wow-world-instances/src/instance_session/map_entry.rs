// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map selection/admission and final Player attachment are separate synchronous phases.
//! C++ MapManager::CreateMap (MapManager.cpp:139-231) returns a map;
//! Player/WorldSession owns the subsequent binding and AddPlayerToMap.
//! The compatibility facade preserves the existing Rust order and failure results.
//! (#1363 slice 13a; moved from the World `session/lifecycle/map_entry.rs`,
//! with the canonical Player residence from `session/instances/map_resolution.rs`
//! and the decision difficulty from `session/map_admission.rs`.)
use std::sync::Arc;
use tracing::warn;
use wow_core::Position;
use wow_world_core::session::connection_identity::unix_now;

use super::{
    InstanceSessionHostLikeCpp, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
    TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP,
    TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP,
    apply_create_map_side_effects_like_cpp, create_map_active_instance_lock_context_like_cpp,
    create_map_player_context_like_cpp,
};

fn create_map_decision_difficulty_id_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::Difficulty> {
    match decision {
        wow_map::CreateMapDecision::Existing { difficulty_id, .. }
        | wow_map::CreateMapDecision::Create { difficulty_id, .. } => Some(*difficulty_id),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

/// Resolve the one canonical Player identity and move that exact value to
/// the selected map. Existing map records predate Player handles, so the
/// transition must adopt them before considering a new initial value.
pub fn ensure_canonical_player_owner_for_map_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    key: wow_map::MapKey,
    position: Position,
) -> bool {
    if !host.ensure_canonical_player_owner_exists_like_cpp(key) {
        return false;
    }
    let Some(manager) = host.core().canonical_map_manager.as_ref().map(Arc::clone) else {
        return false;
    };

    let Some(handle) = host.core().player_handle_like_cpp else {
        return false;
    };
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    match manager.player_residence_like_cpp(handle) {
        Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => manager
            .relocate_player_like_cpp(handle, position)
            .is_ok_and(|outcome| outcome.relocated),
        Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
            manager.detach_player_like_cpp(handle).is_ok()
                && manager
                    .attach_player_like_cpp(handle, key, position)
                    .is_ok()
        }
        Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
            .attach_player_like_cpp(handle, key, position)
            .is_ok(),
        None => false,
    }
}

/// Synchronous entry attempt. Failure retains the source coordinates and
/// transfer state; preparation can still create maps/admission side effects.
/// Full C++ before-add publication ordering and homebind recovery remain
/// separate obligations; this is not a reservation across asynchronous work.
pub fn try_attach_worldport_destination_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: wow_core::Position,
) -> bool {
    try_attach_worldport_destination_with_publication_like_cpp(host, map_id, position, true)
}

pub fn try_attach_worldport_destination_with_publication_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: wow_core::Position,
    publish: bool,
) -> bool {
    if host
        .hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .is_some_and(|state| state.recovery == wow_entities::PlayerTransferRecovery::Terminal)
    {
        return false;
    }
    let Ok(map_id_u16) = u16::try_from(map_id) else {
        return false;
    };
    if !position.is_valid_map_coord_like_cpp() {
        return false;
    }
    let Some(
        wow_map::CreateMapDecision::Existing { key, .. }
        | wow_map::CreateMapDecision::Create { key, .. },
    ) = prepare_canonical_map_entry_with_publication_like_cpp(host, map_id, publish)
    else {
        return false;
    };
    if !host
        .hub_mut()
        .remove_current_player_from_canonical_current_map_like_cpp()
        || !ensure_canonical_player_owner_for_map_like_cpp(host, key, position)
    {
        return false;
    }
    host.hub_mut()
        .set_player_map_position_like_cpp(map_id_u16, position);
    let _ = host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.recovery = wow_entities::PlayerTransferRecovery::None;
        });
    true
}

pub fn ensure_canonical_world_map_for_current_player_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
) -> Option<wow_map::CreateMapDecision> {
    let map_id = u32::from(host.core().player_map_id_like_cpp());
    let position = host.hub_ref().player_position_like_cpp()?;
    let decision = prepare_canonical_map_entry_like_cpp(host, map_id)?;
    if let wow_map::CreateMapDecision::Existing { key, .. }
    | wow_map::CreateMapDecision::Create { key, .. } = &decision
        && !ensure_canonical_player_owner_for_map_like_cpp(host, *key, position)
    {
        return None;
    }
    Some(decision)
}

/// Prepare/create the destination and its existing admission side effects,
/// without relocating or attaching Player. This is not a reservation or a
/// pure query: callers must not carry this decision across asynchronous work.
pub fn prepare_canonical_map_entry_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
) -> Option<wow_map::CreateMapDecision> {
    prepare_canonical_map_entry_with_publication_like_cpp(host, map_id, true)
}

fn prepare_canonical_map_entry_with_publication_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    publish: bool,
) -> Option<wow_map::CreateMapDecision> {
    let map_entry = host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()?
        .get(map_id)
        .copied()?;
    if map_entry.is_battleground_or_arena() {
        return None;
    }
    // C++ `Player::LoadFromDB` rejects a saved map requiring a newer
    // client expansion before calling `MapManager::CreateMap`
    // (Player.cpp:17577-17587). The login handler observes the missing
    // canonical key and performs the same homebind recovery.
    if host.core().expansion < map_entry.expansion_like_cpp() {
        warn!(
            account = host.core().account_id,
            map_id,
            session_expansion = host.core().expansion,
            required_expansion = map_entry.expansion_like_cpp(),
            "Login map rejected by C++ client expansion gate"
        );
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    let player_guid = host.core().player_guid?;
    let player = create_map_player_context_like_cpp(host, map_id, map_entry, player_guid)?;
    let is_dungeon = map_entry.is_dungeon();
    let requested_difficulty = player
        .group
        .map(|group| group.difficulty_id)
        .unwrap_or(player.player_difficulty_id);
    if is_dungeon && {
        let (state, hub) = host.split_instances_ref();
        state.create_map_db2_entries_like_cpp(hub, map_id, requested_difficulty)
    }
    .is_none()
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(map_id, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP);
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }
    let bypass_player_cannot_enter_like_cpp =
        is_dungeon && host.hub_ref().player_is_game_master_like_cpp() == Some(true);
    if is_dungeon
        && !bypass_player_cannot_enter_like_cpp
        && let Some((transfer_abort, arg, map_difficulty_x_condition_id)) =
            host.access_requirement_abort_like_cpp(map_id, requested_difficulty as u8)
    {
        if publish {
            host.hub_ref().send_transfer_aborted_with_params_like_cpp(
                map_id,
                transfer_abort,
                arg,
                map_difficulty_x_condition_id,
            );
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }
    if is_dungeon
        && !bypass_player_cannot_enter_like_cpp
        && map_entry.instance_type == wow_data::map::MAP_RAID
        && map_entry.expansion_like_cpp() >= host.core().realm_policy.server_expansion_like_cpp
        && !host.core().realm_policy.instance_ignore_raid_like_cpp
        && !host.current_player_is_in_raid_group_like_cpp()
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(map_id, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP);
        }
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
            create_map_active_instance_lock_context_like_cpp(host, map_id, requested_difficulty)
        })
        .flatten();

    let manager = Arc::clone(host.core().canonical_map_manager.as_ref()?);
    let mut manager = manager.lock().ok()?;
    let decision = manager.create_map_decision_like_cpp(
        Some(entry),
        Some(player),
        |map_id, difficulty_id| {
            let (state, hub) = host.split_instances_ref();
            state.create_map_difficulty_context_like_cpp(hub, map_id, difficulty_id)
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

    // C++ `MapManager::CreateMap` receives the live `Player*` before it
    // performs admission checks and instance-entry accounting. Materialize
    // that canonical owner before any check can read or mutate
    // `_instanceResetTimes`; the Session retains only a test fallback.
    if let Some(key) = key
        && !host.ensure_canonical_player_owner_exists_like_cpp(key)
    {
        return None;
    }

    if is_dungeon
        && !bypass_player_cannot_enter_like_cpp
        && let wow_map::CreateMapDecision::Existing {
            key, difficulty_id, ..
        } = &decision
        && let Some(player_count) = existing_instance_player_count
        && let Some(entries) = {
            let (state, hub) = host.split_instances_ref();
            state.create_map_db2_entries_like_cpp(hub, key.map_id, *difficulty_id)
        }
        && player_count >= entries.max_players
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(key.map_id, TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP);
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    if is_dungeon
        && !bypass_player_cannot_enter_like_cpp
        && map_entry.instance_type == wow_data::map::MAP_RAID
        && host.player_loading() != Some(player_guid)
        && let wow_map::CreateMapDecision::Existing { key, .. } = &decision
        && existing_instance_encounter_in_progress == Some(true)
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(key.map_id, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP);
        }
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
        let deny_reason = {
            let (state, hub) = host.split_instances_ref();
            state.cannot_enter_existing_instance_lock_like_cpp(
                hub,
                key.map_id,
                *difficulty_id,
                lock_context,
            )
        }
        .unwrap_or(wow_instances::TransferAbortReason::None);
        if deny_reason != wow_instances::TransferAbortReason::None {
            if publish {
                host.hub_ref()
                    .send_transfer_aborted_like_cpp(key.map_id, deny_reason as u32);
            }
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }
    }

    if is_dungeon
        && !bypass_player_cannot_enter_like_cpp
        && !map_entry.ignores_instance_farm_limit_like_cpp()
        && let Some(key) = key
        && !{
            let (state, mut hub) = host.split_instances_mut();
            state.check_instance_count_like_cpp(&mut hub, key.instance_id)
        }
        && host.hub_ref().resolved_player_is_alive_like_cpp() == Some(true)
    {
        if publish {
            host.hub_ref().send_transfer_aborted_like_cpp(
                key.map_id,
                TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP,
            );
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    if is_dungeon && let Some(key) = key {
        let now_secs = u64::try_from(unix_now()).unwrap_or(0);
        {
            let (state, mut hub) = host.split_instances_mut();
            state.add_instance_enter_time_like_cpp(&mut hub, key.instance_id, now_secs)
        };
    }

    let _ = apply_create_map_side_effects_like_cpp(host, map_id, &decision);

    if is_dungeon
        && let Some(key) = key
        && let Some(difficulty_id) = create_map_decision_difficulty_id_like_cpp(&decision)
        && let Some(lock) =
            create_map_active_instance_lock_context_like_cpp(host, map_id, difficulty_id)
    {
        let manager = Arc::clone(host.core().canonical_map_manager.as_ref()?);
        if let Ok(mut manager) = manager.lock()
            && let Some(map) = manager.find_map_mut(key.map_id, key.instance_id)
        {
            map.set_instance_lock_context(Some(lock));
        }
    }

    Some(decision)
}
